use bytes::{BufMut, Bytes};

#[derive(Debug, Clone)]
pub enum Command {
    EzspFrame(Bytes),
    BootloaderFrame(Bytes),
    SpiStatus,
    SpiProtocolVersion,
}

impl Command {
    pub fn size(&self) -> usize {
        match self {
            Command::EzspFrame(b) | Command::BootloaderFrame(b) => 3 + b.len(),
            Command::SpiStatus | Command::SpiProtocolVersion => 2,
        }
    }

    fn command_byte(&self) -> u8 {
        match self {
            Command::EzspFrame(_) => 0xFE,
            Command::BootloaderFrame(_) => 0xFD,
            Command::SpiStatus => 0x0B,
            Command::SpiProtocolVersion => 0x0A,
        }
    }

    pub fn serialize(&self, mut buf: &mut [u8]) {
        buf.put_u8(self.command_byte());
        if let Command::EzspFrame(b) | Command::BootloaderFrame(b) = self {
            buf.put_u8(b.len().try_into().unwrap());
            buf.put_slice(b);
        }
        buf.put_u8(0xA7);
    }
}

#[cfg(test)]
mod tests {
    use bytes::BytesMut;

    use super::*;

    #[test]
    fn it_returns_the_correct_command_size() {
        let data = BytesMut::zeroed(25).freeze();
        assert_eq!(Command::BootloaderFrame(data.clone()).size(), 28);
        assert_eq!(Command::EzspFrame(data).size(), 28);
        assert_eq!(Command::SpiProtocolVersion.size(), 2);
        assert_eq!(Command::SpiStatus.size(), 2);
    }

    #[test]
    fn it_returns_the_correct_command_byte() {
        assert_eq!(Command::BootloaderFrame(Bytes::new()).command_byte(), 0xFD);
        assert_eq!(Command::EzspFrame(Bytes::new()).command_byte(), 0xFE);
        assert_eq!(Command::SpiProtocolVersion.command_byte(), 0x0A);
        assert_eq!(Command::SpiStatus.command_byte(), 0x0B);
    }

    #[test]
    fn it_serialize_a_bootloader_frame_correctly() {
        let command = Command::BootloaderFrame(Bytes::from_static(&[0xA7, 0xFE, 0x0B]));
        let mut buf = BytesMut::zeroed(command.size());
        command.serialize(&mut buf);

        assert_eq!(buf, [0xFD, 0x03, 0xA7, 0xFE, 0x0B, 0xA7].as_ref());
    }

    #[test]
    fn it_serialize_an_ezsp_frame_correctly() {
        let command = Command::EzspFrame(Bytes::from_static(&[0xA7, 0xFE, 0x0B]));
        let mut buf = BytesMut::zeroed(command.size());
        command.serialize(&mut buf);

        assert_eq!(buf, [0xFE, 0x03, 0xA7, 0xFE, 0x0B, 0xA7].as_ref());
    }

    #[test]
    fn it_serialize_the_spi_protocol_version_command_correctly() {
        let command = Command::SpiProtocolVersion;
        let mut buf = BytesMut::zeroed(command.size());
        command.serialize(&mut buf);

        assert_eq!(buf, [0x0A, 0xA7].as_ref());
    }

    #[test]
    fn it_serialize_the_spi_status_command_correctly() {
        let command = Command::SpiStatus;
        let mut buf = BytesMut::zeroed(command.size());
        command.serialize(&mut buf);

        assert_eq!(buf, [0x0B, 0xA7].as_ref());
    }
}
