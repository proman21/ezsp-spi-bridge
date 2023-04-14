use nom::error::{Error as NomError, ErrorKind};
use std::io::Error as IoError;

#[derive(Debug)]
pub enum Error {
    Incomplete,
    InvalidChecksum,
    InvalidDataField,
    Io(IoError),
    UnknownFrame,
}

impl From<NomError<&[u8]>> for Error {
    fn from(value: NomError<&[u8]>) -> Self {
        match value.code {
            ErrorKind::Eof | ErrorKind::Verify => Error::InvalidDataField,
            _ => Error::UnknownFrame,
        }
    }
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
