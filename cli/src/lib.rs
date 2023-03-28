use clap::Parser;

mod cli;
mod error;

type Result<T> = std::result::Result<T, error::Error>;

pub fn run() {
    let cli = cli::Cli::parse();
    cli.run();
}
