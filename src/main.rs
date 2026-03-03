mod cli;

use clap::Parser;
use cli::Cli;
use r3solvr::{BasicResolver, Query, SymbolResolver};
use std::process;

fn main() {
    let cli = Cli::parse();

    if let Err(err) = run(cli) {
        eprintln!("Error: {}", err);
        process::exit(1);
    }
}

fn display_stripped(stripped: bool) -> &'static str {
    if stripped { "S" } else { "V" }
}

fn run(cli: Cli) -> r3solvr::ResolverResult<()> {
    let resolver = BasicResolver::from_file(&cli.file)?;

    match &cli.query {
        Some(query) => {
            let config = Query::new(query)
                .with_prefix(cli.prefix)
                .with_debugdata(cli.debugdata);

            let symbol = resolver.lookup_symbol(config)?;

            println!(
                "{:x}\t{}\t{} {}",
                symbol.addr,
                symbol.section_index,
                display_stripped(symbol.stripped),
                symbol.name
            );
        }
        None => {
            for symbol in resolver.list_symbols(cli.debugdata) {
                println!(
                    "{:x}\t{}\t{} {}",
                    symbol.addr,
                    symbol.section_index,
                    display_stripped(symbol.stripped),
                    symbol.name
                );
            }
        }
    }

    Ok(())
}
