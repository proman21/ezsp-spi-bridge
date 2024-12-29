use std::{io::Error as IoError, result::Result as StdResult};
use thiserror::Error;
use tokio::sync::mpsc::error::SendError;

use super::frame::Frame;

#[derive(Debug, Error)]
pub enum Error {
    #[error("A frame was received with an invalid checksum: {0}")]
    InvalidChecksum(Frame),
    #[error("A frame was received with an invalid data field: {0}")]
    InvalidDataField(Frame),
    #[error("An IO error occurred")]
    Io(#[from] IoError),
    #[error("An unknown frame type was encountered")]
    UnknownFrame,
    #[error("An error occurred while sending a frame")]
    Channel(#[from] SendError<Frame>)
}

impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        core::mem::discriminant(self) == core::mem::discriminant(other)
    }
}

pub type Result<T> = StdResult<T, Error>;
