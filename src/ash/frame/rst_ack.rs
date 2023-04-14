use bytes::BufMut;
use nom::{
    bytes::complete::tag,
    number::complete::u8,
    sequence::{preceded, tuple},
};

use super::FrameFormat;

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

    fn serialize_data(&self, mut buf: &mut [u8]) {
        buf.put_u8(self.version);
        buf.put_u8(self.code);
    }

    fn parse(input: &[u8]) -> super::ParserResult<Self> {
        let (rest, (version, code)) = preceded(tag([0xC1]), tuple((u8, u8)))(input)?;
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
        let buf = [0xC1, 0x02, 0x02];
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
        let mut buf = BytesMut::zeroed(2);

        frame.serialize_data(&mut buf);
        assert_eq!(*buf, [0x02, 0x02]);
    }
}
