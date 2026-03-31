mod resolver;
mod result;
pub use resolver::*;
pub use result::*;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: Box<str>,
    pub addr: usize,
    pub section_index: usize,
    pub stripped: bool,
}

#[derive(Debug, Clone)]
pub struct Section {
    pub name: Box<str>,
    pub addr: usize,
    pub file_range: Option<(usize, usize)>,
}

#[derive(Copy, Clone)]
pub struct Query<'q> {
    pattern: &'q str,
    prefix: bool,
    debugdata: bool,
}

impl<'q> Query<'q> {
    pub fn new(query: &'q str) -> Self {
        Self {
            pattern: query,
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

    fn matches(&self, name: &str) -> bool {
        if self.prefix {
            name.starts_with(self.pattern)
        } else {
            name == self.pattern
        }
    }
}

impl<'a> From<&'a str> for Query<'a> {
    fn from(value: &'a str) -> Self {
        Self {
            pattern: value,
            prefix: false,
            debugdata: false,
        }
    }
}

pub trait SymbolResolver: Sized {
    type ResolverImpl;

    fn from_file<P: AsRef<Path>>(file: P) -> ResolverResult<Self::ResolverImpl> {
        Self::from_data(fs::read(file)?)
    }

    fn from_data(data: Vec<u8>) -> ResolverResult<Self::ResolverImpl>;

    fn lookup_symbol<'q, Q: Into<Query<'q>>>(&self, query: Q) -> ResolverResult<Symbol>;

    fn lookup_section(&self, index: usize) -> ResolverResult<Section>;
}
