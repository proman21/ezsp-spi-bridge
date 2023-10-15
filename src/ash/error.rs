use std::io::Error as IoError;

#[derive(Debug)]
pub enum Error {
    InvalidChecksum,
    InvalidDataField,
    Io(IoError),
    UnknownFrame,
}

impl From<IoError> for Error {
    fn from(value: IoError) -> Self {
        Error::Io(value)
    }
}

impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        core::mem::discriminant(self) == core::mem::discriminant(other)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
