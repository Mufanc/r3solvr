use crate::result::{ResolverError, ResolverResult};
use object::{Object, ObjectSection, ObjectSymbol, SectionIndex};
use std::fs;
use std::marker::PhantomPinned;
use std::mem::MaybeUninit;
use std::path::Path;
use std::pin::Pin;
use std::sync::OnceLock;

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: Box<str>,
    pub addr: usize,
    pub section_index: usize,
}

#[derive(Debug, Clone)]
pub struct Section {
    pub name: Box<str>,
    pub addr: usize,
    pub file_range: Option<(usize, usize)>,
}

#[derive(Copy, Clone)]
pub struct LookupConfig<'a> {
    pub query: &'a str,
    pub prefix: bool,
    pub debugdata: bool,
}

impl<'a> LookupConfig<'a> {
    pub fn new(query: &'a str) -> Self {
        Self {
            query,
            prefix: false,
            debugdata: false,
        }
    }

    pub fn with_prefix(mut self, prefix: bool) -> Self {
        self.prefix = prefix;
        self
    }

    pub fn with_debugdata(mut self, debugdata: bool) -> Self {
        self.debugdata = debugdata;
        self
    }
}

pub struct SymbolResolver<'a> {
    data: Box<[u8]>,
    file: MaybeUninit<object::File<'a>>,
    debugdata_resolver: OnceLock<Option<Pin<Box<SymbolResolver<'static>>>>>,
    _pin: PhantomPinned,
}

fn not_found_msg(config: &LookupConfig) -> String {
    if config.prefix {
        format!("cannot find symbol with prefix: {}", config.query)
    } else {
        format!("cannot find symbol: {}", config.query)
    }
}

impl SymbolResolver<'_> {
    pub fn from_file<P: AsRef<Path>>(file: P) -> ResolverResult<Pin<Box<Self>>> {
        Self::from_data(fs::read(file)?)
    }

    pub fn from_data(data: Vec<u8>) -> ResolverResult<Pin<Box<Self>>> {
        let mut pinned = Box::pin(Self {
            data: data.into_boxed_slice(),
            file: MaybeUninit::uninit(),
            debugdata_resolver: OnceLock::new(),
            _pin: PhantomPinned,
        });

        unsafe {
            let ptr = pinned.as_mut().get_unchecked_mut() as *mut SymbolResolver;
            let refr = &mut *ptr;

            refr.file.write(object::File::parse(&*refr.data)?);
        }

        Ok(pinned)
    }

    pub fn lookup_symbol(&self, config: LookupConfig) -> ResolverResult<Symbol> {
        let file = self.file();

        let result: Option<_> = file
            .dynamic_symbols()
            .chain(file.symbols())
            .find_map(|sym| {
                sym.name()
                    .ok()
                    .filter(|name| {
                        if config.prefix {
                            name.starts_with(config.query)
                        } else {
                            *name == config.query
                        }
                    })
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

        if !config.debugdata {
            return Err(ResolverError::NotFound(not_found_msg(&config)));
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
                .and_then(|data| SymbolResolver::from_data(data).ok())
        });

        let result = debugdata_resolver.as_ref().and_then(|resolver| {
            resolver
                .lookup_symbol(LookupConfig::new(config.query).with_prefix(config.prefix))
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

        result.ok_or_else(|| ResolverError::NotFound(not_found_msg(&config)))
    }

    pub fn lookup_section(&self, index: usize) -> ResolverResult<Section> {
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

impl SymbolResolver<'_> {
    fn file(&self) -> &object::File<'_> {
        unsafe { self.file.assume_init_ref() }
    }
}

impl Drop for SymbolResolver<'_> {
    fn drop(&mut self) {
        unsafe {
            self.file.assume_init_drop();
        }
    }
}
