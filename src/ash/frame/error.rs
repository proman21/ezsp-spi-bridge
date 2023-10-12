use bytes::{Buf, BufMut, BytesMut};
use nom::{
    bytes::streaming::tag,
    error::{Error, ErrorKind},
    Err,
};

use crate::ash::checksum::crc_digester;

use super::utils::{frame_data_and_flag, FrameFormat, ParserResult};

#[derive(Debug)]
pub struct ErrorFrame {
    version: u8,
    code: u8,
}

impl ErrorFrame {
    pub fn new(version: u8, code: u8) -> ErrorFrame {
        ErrorFrame { version, code }
    }

    pub fn version(&self) -> u8 {
        self.version
    }

    pub fn code(&self) -> u8 {
        self.code
    }
}

impl FrameFormat for ErrorFrame {
    fn flag(&self) -> u8 {
        0xC2
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
        let (i2, ctrl) = tag([0xC2])(input)?;
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

        let frame = ErrorFrame::new(version, code);
        Ok((rest, frame))
    }
}

#[cfg(test)]
mod tests {
    use super::ErrorFrame;
    use crate::ash::frame::FrameFormat;
    use bytes::BytesMut;

    #[test]
    fn it_parses_a_valid_frame_correctly() {
        let buf = [0xC2, 0x02, 0x51, 0xA8, 0xBD, 0x7E];
        let (_rest, frame) = ErrorFrame::parse(&buf).unwrap();

        assert_eq!(frame.version(), 0x02);
        assert_eq!(frame.code(), 0x51);
    }

    #[test]
    fn it_rejects_an_invalid_frame() {
        let buf = [0xC2];
        let res = ErrorFrame::parse(&buf);

        assert!(res.is_err());
    }

    #[test]
    fn it_rejects_a_early_terminated_frame() {
        let buf = [0xC2, 0x02, 0x51, 0x7E];
        let res = ErrorFrame::parse(&buf);

        assert!(res.is_err());
    }

    #[test]
    fn it_serializes_the_control_byte_correctly() {
        let frame = ErrorFrame::new(0x02, 0x52);

        assert_eq!(frame.flag(), 0xC2);
    }

    #[test]
    fn it_returns_correct_data_field_len() {
        let frame = ErrorFrame::new(0x02, 0x52);

        assert_eq!(frame.data_len(), 2);
    }

    #[test]
    fn it_serializes_data_field_correctly() {
        let frame = ErrorFrame::new(0x02, 0x52);
        let mut buf = BytesMut::with_capacity(2);

        frame.serialize_data(&mut buf);
        assert_eq!(*buf, [0x02, 0x52]);
    }
}
