# r3solvr

A lightweight ELF symbol resolver with MiniDebugInfo support.

## Features

- Parse symbols from ELF dynamic symbol table and symbol table
- Decompress and read `.gnu_debugdata` section for stripped binaries
- Prefix-based symbol matching
- Optional query result caching via `CachedResolver`

## Installation

```bash
cargo install --path . --features cli
```

Or build from source:

```bash
cargo build --release --features cli
```

## Usage

### Command Line

```bash
# List all symbols
r3solvr /path/to/binary

# Lookup a specific symbol
r3solvr /path/to/binary symbol_name

# Lookup with prefix matching
r3solvr --prefix /path/to/binary func_

# Include symbols from .gnu_debugdata
r3solvr --debugdata /path/to/binary
```

### Output Format

```
<address>	<section_index>	<V|S> <symbol_name>
```

- `V` - Symbol from visible symbol table
- `S` - Symbol from stripped debugdata

### Library

```rust
use r3solvr::{BasicResolver, Query, SymbolResolver};

let resolver = BasicResolver::from_file("/path/to/binary")?;

// Simple lookup
let symbol = resolver.lookup_symbol("main")?;

// Query with options
let query = Query::new("init")
    .with_prefix(true)
    .with_debugdata(true);
let symbol = resolver.lookup_symbol(query)?;

println!("{}: 0x{:x}", symbol.name, symbol.addr);
```
