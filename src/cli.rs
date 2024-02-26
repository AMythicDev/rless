use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct CommandLineInterface {
    #[arg()]
    pub filenames: Vec<PathBuf>,
    #[arg(short, long)]
    pub buffers: Option<isize>,
    /// Automatically quit when all files have been done viewing. By default you can quit only
    /// using the "q" key.
    #[arg(short = 'e', long)]
    pub quit_on_eof: bool,
    /// Whether to use incremental search for searches
    #[arg(long = "incsearch")]
    pub incsearch: bool,
}
