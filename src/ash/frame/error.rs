use bytes::BufMut;
use nom::{
    bytes::complete::tag,
    number::complete::u8,
    sequence::{preceded, tuple},
};

use super::FrameFormat;

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

    fn serialize_data(&self, mut buf: &mut [u8]) {
        buf.put_u8(self.version);
        buf.put_u8(self.code);
    }

    fn parse(input: &[u8]) -> super::ParserResult<Self> {
        let (rest, (version, code)) = preceded(tag([0xC2]), tuple((u8, u8)))(input)?;
        let frame = ErrorFrame::new(version, code);
        Ok((rest, frame))
    }
}

#[cfg(test)]
mod tests {
    use bytes::BytesMut;

    use crate::ash::frame::FrameFormat;

    use super::ErrorFrame;

    #[test]
    fn it_parse_a_valid_frame_correctly() {
        let buf = [0xC2, 0x01, 0x52];
        let (_rest, frame) = ErrorFrame::parse(&buf).unwrap();

        assert_eq!(frame.version(), 0x01);
        assert_eq!(frame.code(), 0x52);
    }

    #[test]
    fn it_fails_to_parse_invalid_frame() {
        let buf = [0xC2];
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
        let mut buf = BytesMut::zeroed(2);

        frame.serialize_data(&mut buf);
        assert_eq!(*buf, [0x02, 0x52]);
    }
}
