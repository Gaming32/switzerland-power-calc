use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_cbor::Error),
    #[error("CLI error: {0}")]
    Readline(#[from] rustyline::error::ReadlineError),
}

pub type Result<T> = std::result::Result<T, Error>;
