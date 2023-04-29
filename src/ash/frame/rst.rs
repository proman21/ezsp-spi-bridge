use nom::bytes::complete::tag;

use crate::buffer::{BufferMut};

use super::{FrameFormat, ParserResult};

#[derive(Debug)]
pub struct RstFrame;

impl FrameFormat for RstFrame {
    fn flag(&self) -> u8 {
        0xC0
    }

    fn parse(input: BufferMut) -> ParserResult<Self> {
        let (rest, _control) = tag([0xC0])(input)?;
        Ok((rest, RstFrame))
    }
}

#[cfg(test)]
mod tests {
    use bytes::BytesMut;

    use crate::ash::frame::FrameFormat;

    use super::RstFrame;

    #[test]
    fn it_serializes_the_control_byte_correctly() {
        let frame = RstFrame;

        assert_eq!(frame.flag(), 0xC0)
    }

    #[test]
    fn it_returns_correct_data_field_len() {
        let frame = RstFrame;

        assert_eq!(frame.data_len(), 0);
    }

    #[test]
    fn it_serializes_data_field_correctly() {
        let frame = RstFrame;
        let mut buf = BytesMut::zeroed(2);

        frame.serialize_data(&mut buf);
        assert_eq!(*buf, [0; 2])
    }
}
