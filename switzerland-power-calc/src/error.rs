use std::backtrace::Backtrace;
use std::fmt::{Debug, Display, Formatter};
use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ErrorKind {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("CBOR Serialization error: {0}")]
    CborSerialization(#[from] serde_cbor::Error),
    #[error("JSON Serialization error: {0}")]
    JsonSerialization(#[from] serde_json::Error),
    #[error("CLI error: {0}")]
    Readline(#[from] rustyline_async::ReadlineError),
    #[error("Missing environment variable {0}")]
    MissingEnv(String),
    #[error("Invalid environment variable {0}: {1}")]
    InvalidEnv(String, #[source] Box<dyn std::error::Error + Send>),
    #[error("Invalid logging environment: {0}")]
    InvalidLogEnv(#[from] tracing_subscriber::filter::FromEnvError),
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("URL parsing error: {0}")]
    Url(#[from] url::ParseError),
    #[error("Discord error: {0}")]
    Discord(#[source] Box<serenity::Error>),
    #[error("Animation error: {0}")]
    Animation(#[from] switzerland_power_animated::Error),
    #[error("{0}")]
    Custom(String),
}

impl From<serenity::Error> for ErrorKind {
    fn from(value: serenity::Error) -> Self {
        ErrorKind::Discord(Box::new(value))
    }
}

impl From<String> for ErrorKind {
    fn from(value: String) -> Self {
        Self::Custom(value)
    }
}

impl From<&str> for ErrorKind {
    fn from(value: &str) -> Self {
        Self::Custom(value.to_string())
    }
}

#[derive(Debug)]
pub struct Error {
    pub error: ErrorKind,
    pub backtrace: Backtrace,
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}

impl<E: Into<ErrorKind>> From<E> for Error {
    fn from(source: E) -> Self {
        Self {
            error: source.into(),
            backtrace: Backtrace::capture(),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.error, f)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
