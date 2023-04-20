#[derive(Debug)]
pub enum Error {
    NcpReset(u8),
    OversizedPayloadFrame,
    AbortedTransaction,
    MissingFrameTerminator,
    UnsupportedSpiCommand,
    Io(std::io::Error),
    Unresponsive
}

pub type Result<T> = std::result::Result<T, Error>;
