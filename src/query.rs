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
