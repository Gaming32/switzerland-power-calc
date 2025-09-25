use webp::AnimEncodeError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("SDL error: {0}")]
    Sdl(String),
    #[error("WebP encode error: {0:?}")]
    WebP(AnimEncodeError),
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<String> for Error {
    fn from(value: String) -> Self {
        Self::Sdl(value)
    }
}

impl From<AnimEncodeError> for Error {
    fn from(value: AnimEncodeError) -> Self {
        Error::WebP(value)
    }
}
