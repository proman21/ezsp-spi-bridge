use bytes::{Buf, BufMut, BytesMut};
use nom::{
    bytes::streaming::tag,
    error::{Error, ErrorKind},
    Err,
};

use crate::ash::checksum::crc_digester;

use super::utils::{frame_data_and_flag, FrameFormat, ParserResult};

#[derive(Debug)]
pub struct RstAckFrame {
    version: u8,
    code: u8,
}

impl RstAckFrame {
    pub fn new(version: u8, code: u8) -> RstAckFrame {
        RstAckFrame { version, code }
    }

    pub fn version(&self) -> u8 {
        self.version
    }

    pub fn code(&self) -> u8 {
        self.code
    }
}

impl FrameFormat for RstAckFrame {
    fn flag(&self) -> u8 {
        0xC1
    }

    fn data_len(&self) -> usize {
        2
    }

    fn serialize_data(&self, buf: &mut BytesMut) {
        buf.reserve(2);
        buf.put_u8(self.version);
        buf.put_u8(self.code);
    }

    fn parse(input: &[u8]) -> ParserResult<Self> {
        let mut crc = crc_digester();

        let (i2, ctrl) = tag([0xC1])(input)?;
        crc.update(ctrl);

        let (rest, mut buf) = frame_data_and_flag(i2)?;
        if buf.len() != 4 {
            return Err(Err::Failure(Error::new(rest, ErrorKind::Complete)));
        }

        crc.update(&buf[..2]);
        let version = buf.get_u8();
        let code = buf.get_u8();
        let checksum = buf.get_u16();
        if checksum != crc.finalize() {
            return Err(Err::Failure(Error::new(rest, ErrorKind::Verify)));
        }

        let frame = RstAckFrame::new(version, code);
        Ok((rest, frame))
    }
}

#[cfg(test)]
mod tests {
    use bytes::BytesMut;

    use crate::ash::frame::FrameFormat;

    use super::RstAckFrame;

    #[test]
    fn it_parse_a_valid_frame_correctly() {
        let buf = [0xC1, 0x02, 0x02, 0x9B, 0x7B, 0x7E];
        let (_rest, frame) = RstAckFrame::parse(&buf).unwrap();

        assert_eq!(frame.version(), 0x02);
        assert_eq!(frame.code(), 0x02);
    }

    #[test]
    fn it_fails_to_parse_invalid_frame() {
        let buf = [0xC1];
        let res = RstAckFrame::parse(&buf);

        assert!(res.is_err());
    }

    #[test]
    fn it_serializes_the_control_byte_correctly() {
        let frame = RstAckFrame::new(0x02, 0x02);

        assert_eq!(frame.flag(), 0xC1);
    }

    #[test]
    fn it_returns_correct_data_field_len() {
        let frame = RstAckFrame::new(0x02, 0x02);

        assert_eq!(frame.data_len(), 2);
    }

    #[test]
    fn it_serializes_data_field_correctly() {
        let frame = RstAckFrame::new(0x02, 0x02);
        let mut buf = BytesMut::with_capacity(2);

        frame.serialize_data(&mut buf);
        assert_eq!(*buf, [0x02, 0x02]);
    }
}
