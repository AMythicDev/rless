use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct CommandLineInterface {
    #[arg()]
    pub filename: Vec<PathBuf>,
    #[arg(short, long)]
    pub buffers: Option<isize>,
}
