use sdl2::ttf::FontError;
use std::ffi::NulError;
use webp::AnimEncodeError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("SDL error: {0}")]
    Sdl(String),
    #[error("WebP encode error: {0:?}")]
    WebP(AnimEncodeError),
    #[error("Invalid CString: {0}")]
    CString(#[from] NulError),
    #[error("Invalid status: {0}")]
    InvalidStatus(String),
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

impl From<FontError> for Error {
    fn from(value: FontError) -> Self {
        match value {
            FontError::InvalidLatin1Text(e) => Error::CString(e),
            FontError::SdlError(e) => Error::Sdl(e),
        }
    }
}
