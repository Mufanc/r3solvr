use crate::result::{ResolverError, ResolverResult};
use crate::{Query, Section, Symbol, SymbolResolver};
use object::{Object, ObjectSection, ObjectSymbol, SectionIndex};
use once_map::OnceMap;
use std::marker::PhantomPinned;

use std::pin::Pin;
use std::{iter, slice};
use std::sync::OnceLock;

pub type PinnedBasicResolver = Pin<Box<BasicResolver>>;

pub struct BasicResolver {
    data: Box<[u8]>,
    file: Option<object::File<'static>>,
    debugdata_resolver: OnceLock<Option<PinnedBasicResolver>>,
    _pin: PhantomPinned,
}

fn not_found_msg(query: &Query) -> String {
    if query.prefix {
        format!("cannot find symbol with prefix: {}", query.pattern)
    } else {
        format!("cannot find symbol: {}", query.pattern)
    }
}

impl BasicResolver {
    fn file(&self) -> &object::File<'_> {
        // SAFETY: `file` is always `Some` after successful construction.
        // `BasicResolver` can only be created through `from_data`, which sets
        // `file` to `Some` before returning `Ok`.
        unsafe { self.file.as_ref().unwrap_unchecked() }
    }

    fn debugdata_resolver(&self) -> &Option<PinnedBasicResolver> {
        self.debugdata_resolver.get_or_init(move || {
            self.file()
                .section_by_name(".gnu_debugdata")
                .and_then(|sec| sec.data().ok())
                .and_then(|mut data| {
                    let mut decompressed = Vec::new();
                    lzma_rs::xz_decompress(&mut data, &mut decompressed)
                        .ok()
                        .map(|_| decompressed)
                })
                .and_then(|data| Self::from_data(data).ok())
        })
    }

    pub fn list_symbols(&self, debugdata: bool) -> Box<dyn Iterator<Item = Symbol> + '_> {
        let file = self.file();

        let main_symbols = file
            .dynamic_symbols()
            .chain(file.symbols())
            .filter_map(|sym| {
                sym.name().ok().and_then(|name| {
                    if name.is_empty() {
                        return None;
                    }
                    sym.section_index().map(|index| Symbol {
                        name: name.into(),
                        addr: sym.address() as _,
                        section_index: index.0,
                    })
                })
            });

        let debug_symbols: Box<dyn Iterator<Item = Symbol>> = if debugdata {
            if let Some(resolver) = self.debugdata_resolver().as_ref() {
                Box::new(resolver.list_symbols(false).filter_map(move |sym| {
                    resolver
                        .file()
                        .section_by_index(SectionIndex(sym.section_index))
                        .and_then(|sec| sec.name())
                        .ok()
                        .and_then(|name| file.section_by_name(name))
                        .map(|sec| Symbol {
                            name: sym.name,
                            addr: sym.addr,
                            section_index: sec.index().0,
                        })
                }))
            } else {
                Box::new(iter::empty())
            }
        } else {
            Box::new(iter::empty())
        };

        Box::new(main_symbols.chain(debug_symbols))
    }
}

impl SymbolResolver for BasicResolver {
    type ResolverImpl = PinnedBasicResolver;

    fn from_data(data: Vec<u8>) -> ResolverResult<Self::ResolverImpl> {
        let mut boxed = Box::new(BasicResolver {
            data: data.into_boxed_slice(),
            file: None,
            debugdata_resolver: OnceLock::new(),
            _pin: PhantomPinned,
        });

        // SAFETY:
        // 1. `slice::from_raw_parts`: `data_ptr` and `data_len` are derived from
        //    `boxed.data`, which is valid and owned by `boxed`.
        // 2. Lifetime extension to `'static`: The actual lifetime is tied to `boxed.data`.
        //    This is sound because `BasicResolver` is a self-referential struct where:
        //    - `data` owns the bytes and `file` borrows from it
        //    - `PhantomPinned` prevents moving the struct
        //    - Returning `Pin<Box<...>>` ensures the address remains stable
        // 3. `Pin::new_unchecked`: Safe because `Box` provides heap allocation with
        //    a stable address, and `PhantomPinned` enforces pinning guarantees.
        unsafe {
            let data_ptr = boxed.data.as_ptr();
            let data_len = boxed.data.len();

            let file = object::File::parse(slice::from_raw_parts(data_ptr, data_len))?;
            boxed.file = Some(file);

            Ok(Pin::new_unchecked(boxed))
        }
    }

    fn lookup_symbol<'q, Q: Into<Query<'q>>>(&self, query: Q) -> ResolverResult<Symbol> {
        let query = query.into();
        let file = self.file();

        let result: Option<_> = file
            .dynamic_symbols()
            .chain(file.symbols())
            .find_map(|sym| {
                sym.name()
                    .ok()
                    .filter(|name| query.matches(name))
                    .and_then(|name| {
                        sym.section_index().map(|index| Symbol {
                            name: name.into(),
                            addr: sym.address() as _,
                            section_index: index.0,
                        })
                    })
            });

        if let Some(result) = result {
            return Ok(result);
        }

        if !query.debugdata {
            return Err(ResolverError::NotFound(not_found_msg(&query)));
        }

        let result = self.debugdata_resolver().as_ref().and_then(|resolver| {
            resolver
                .lookup_symbol(query.with_debugdata(false))
                .into_iter()
                .find_map(|sym| {
                    let Symbol {
                        name,
                        addr,
                        section_index,
                    } = sym;

                    file.section_by_index(SectionIndex(section_index))
                        .and_then(|sec| sec.name())
                        .ok()
                        .and_then(|name| file.section_by_name(name))
                        .map(|sec| Symbol {
                            name,
                            addr,
                            section_index: sec.index().0,
                        })
                })
        });

        result.ok_or_else(|| ResolverError::NotFound(not_found_msg(&query)))
    }

    fn lookup_section(&self, index: usize) -> ResolverResult<Section> {
        let section = self
            .file()
            .section_by_index(SectionIndex(index))
            .map_err(|_| {
                ResolverError::NotFound(format!("cannot find section with index: {index}"))
            })?;

        Ok(Section {
            name: section.name()?.into(),
            addr: section.address() as _,
            file_range: section.file_range().map(|(s, e)| (s as _, e as _)),
        })
    }
}

#[derive(Eq, PartialEq, Hash)]
struct CacheKey {
    pattern: Box<str>,
    prefix: bool,
    debugdata: bool,
}

pub struct CachedResolver {
    resolver: PinnedBasicResolver,
    caches: OnceMap<CacheKey, Symbol>,
}

impl SymbolResolver for CachedResolver {
    type ResolverImpl = Self;

    fn from_data(data: Vec<u8>) -> ResolverResult<Self> {
        Ok(Self {
            resolver: BasicResolver::from_data(data)?,
            caches: OnceMap::new(),
        })
    }

    fn lookup_symbol<'q, Q: Into<Query<'q>>>(&self, query: Q) -> ResolverResult<Symbol> {
        let query = query.into();
        let key = CacheKey {
            pattern: query.pattern.into(),
            prefix: query.prefix,
            debugdata: query.debugdata,
        };

        self.caches.map_try_insert(
            key,
            |_| self.resolver.lookup_symbol(query),
            |_, v| v.clone(),
        )
    }

    fn lookup_section(&self, index: usize) -> ResolverResult<Section> {
        self.resolver.lookup_section(index)
    }
}
