mod cli;

use clap::Parser;
use cli::Cli;
use r3solvr::{CachedResolver, Query, SymbolResolver};
use std::process;

fn main() {
    let cli = Cli::parse();

    if let Err(err) = run(cli) {
        eprintln!("Error: {}", err);
        process::exit(1);
    }
}

fn run(cli: Cli) -> r3solvr::ResolverResult<()> {
    let resolver = CachedResolver::from_file(&cli.file)?;

    let config = Query::new(&cli.query)
        .with_prefix(cli.prefix)
        .with_debugdata(cli.debugdata);

    let symbol = resolver.lookup_symbol(config)?;

    println!("{}\t{}\t{}", symbol.addr, symbol.section_index, symbol.name);

    Ok(())
}
