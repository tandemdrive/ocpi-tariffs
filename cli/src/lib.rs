#![doc = include_str!("../README.md")]

mod cli;
mod error;

type Result<T> = std::result::Result<T, error::Error>;

pub use cli::Cli;
