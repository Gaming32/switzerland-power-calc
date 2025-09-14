use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("CBOR Serialization error: {0}")]
    CborSerialization(#[from] serde_cbor::Error),
    #[error("JSON Serialization error: {0}")]
    JsonSerialization(#[from] serde_json::Error),
    #[error("CLI error: {0}")]
    Readline(#[from] rustyline::error::ReadlineError),
    #[error("Missing environment variable {0}")]
    MissingEnv(String),
    #[error("Invalid environment variable {0}: {1}")]
    InvalidEnv(String, #[source] Box<dyn std::error::Error>),
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("URL parsing error: {0}")]
    Url(#[from] url::ParseError),
    #[error("Discord error: {0}")]
    Discord(#[source] Box<serenity::Error>),
    #[error("{0}")]
    Custom(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<serenity::Error> for Error {
    fn from(value: serenity::Error) -> Self {
        Self::Discord(Box::new(value))
    }
}
