use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "r3solvr")]
#[command(about = "ELF symbol resolver", long_about = None)]
pub struct Cli {
    #[arg(long)]
    pub prefix: bool,

    #[arg(long)]
    pub debugdata: bool,

    #[arg(index = 1)]
    pub file: PathBuf,

    #[arg(index = 2)]
    pub query: Option<String>,
}
