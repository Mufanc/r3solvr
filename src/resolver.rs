use crate::result::{ResolverError, ResolverResult};
use crate::{Query, Section, Symbol, SymbolResolver};
use object::{Object, ObjectSection, ObjectSymbol, SectionIndex};
use once_map::OnceMap;
use std::marker::PhantomPinned;
use std::mem::MaybeUninit;
use std::pin::Pin;
use std::sync::OnceLock;

pub type PinnedBasicResolver<'a> = Pin<Box<BasicResolver<'a>>>;

pub struct BasicResolver<'a> {
    data: Box<[u8]>,
    file: MaybeUninit<object::File<'a>>,
    debugdata_resolver: OnceLock<Option<PinnedBasicResolver<'a>>>,
    _pin: PhantomPinned,
}

fn not_found_msg(query: &Query) -> String {
    if query.prefix {
        format!("cannot find symbol with prefix: {}", query.pattern)
    } else {
        format!("cannot find symbol: {}", query.pattern)
    }
}

impl BasicResolver<'_> {
    fn file(&self) -> &object::File<'_> {
        unsafe { self.file.assume_init_ref() }
    }
}

impl<'a> SymbolResolver for BasicResolver<'a> {
    type ResolverImpl = PinnedBasicResolver<'a>;

    fn from_data(data: Vec<u8>) -> ResolverResult<Self::ResolverImpl> {
        let mut pinned = Box::pin(BasicResolver {
            data: data.into_boxed_slice(),
            file: MaybeUninit::uninit(),
            debugdata_resolver: OnceLock::new(),
            _pin: PhantomPinned,
        });

        unsafe {
            let ptr = pinned.as_mut().get_unchecked_mut() as *mut BasicResolver;
            let refr = &mut *ptr;

            refr.file.write(object::File::parse(&*refr.data)?);
        }

        Ok(pinned)
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

        let debugdata_resolver = self.debugdata_resolver.get_or_init(|| {
            file.section_by_name(".gnu_debugdata")
                .and_then(|sec| sec.data().ok())
                .and_then(|mut data| {
                    let mut decompressed = Vec::new();
                    lzma_rs::xz_decompress(&mut data, &mut decompressed)
                        .ok()
                        .map(|_| decompressed)
                })
                .and_then(|data| Self::from_data(data).ok())
        });

        let result = debugdata_resolver.as_ref().and_then(|resolver| {
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

impl Drop for BasicResolver<'_> {
    fn drop(&mut self) {
        unsafe {
            self.file.assume_init_drop();
        }
    }
}

#[derive(Eq, PartialEq, Hash)]
struct CacheKey {
    pattern: Box<str>,
    prefix: bool,
    debugdata: bool,
}

pub struct CachedResolver<'a> {
    resolver: PinnedBasicResolver<'a>,
    caches: OnceMap<CacheKey, Symbol>,
}

impl SymbolResolver for CachedResolver<'_> {
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
