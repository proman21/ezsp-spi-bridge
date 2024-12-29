use std::result::Result as StdResult;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("An invalid response was sent")]
    InvalidResponse,
    #[error("An IO error occurred")]
    Io(#[from] std::io::Error),
    #[error("The NCP is in an unknown state")]
    NeedsReset,
    #[error("The NCP is unresponsive")]
    Unresponsive,
    #[error("The NCP received a request payload that exceeds the maximum size")]
    OversizedPayload,
    #[error("An unexpected internal error occurred")]
    InternalError,
    #[error("An unexpected reset condition was encountered: {0}")]
    UnexpectedReset(u8),
}

pub type Result<T> = StdResult<T, Error>;
