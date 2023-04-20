use bytes::Bytes;

#[derive(Debug)]
pub enum Command {
    EzspFrame(Bytes),
    BootloaderFrame(Bytes),
    SpiStatus,
    SpiProtocolVersion,
}
