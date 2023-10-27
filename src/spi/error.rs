#[derive(Debug)]
pub enum Error {
    InvalidResponse,
    Io(std::io::Error),
    NeedsReset,
    Unresponsive,
    OversizedPayload,
    InternalError,
    UnexpectedReset(u8),
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::Io(value)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
