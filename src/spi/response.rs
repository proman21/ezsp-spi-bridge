use bytes::Bytes;

use super::error::Error;

#[derive(Debug)]
pub enum Response {
    EzspFrame(Bytes),
    BootloaderFrame(Bytes),
    SpiStatus(u8),
    SpiProtocolVersion(u8),
    Error(Error),
}
