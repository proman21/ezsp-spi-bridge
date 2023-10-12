use bytes::Buf;
use nom::{
    bytes::streaming::tag,
    error::{Error, ErrorKind},
    Err,
};

use crate::ash::checksum::crc_digester;

use super::utils::{frame_data_and_flag, FrameFormat, ParserResult};

#[derive(Debug)]
pub struct RstFrame;

impl FrameFormat for RstFrame {
    fn flag(&self) -> u8 {
        0xC0
    }

    fn parse(input: &[u8]) -> ParserResult<Self> {
        let mut crc = crc_digester();
        let (i2, ctrl) = tag([0xC0])(input)?;
        crc.update(ctrl);

        let (rest, mut checksum_buf) = frame_data_and_flag(i2)?;
        if checksum_buf.len() != 2 {
            return Err(Err::Failure(Error::new(rest, ErrorKind::Complete)));
        }
        let checksum = checksum_buf.get_u16();
        if crc.finalize() != checksum {
            return Err(Err::Failure(Error::new(rest, ErrorKind::Verify)));
        };

        Ok((rest, RstFrame))
    }
}

#[cfg(test)]
mod tests {
    use bytes::BytesMut;

    use crate::ash::frame::FrameFormat;

    use super::RstFrame;

    #[test]
    fn it_parses_a_valid_frame_correctly() {
        let buf = [0xC0, 0x38, 0xBC, 0x7E];
        let (rest, _frame) = RstFrame::parse(&buf).unwrap();

        assert_eq!(rest.len(), 0);
    }

    #[test]
    fn it_rejects_a_early_terminated_frame() {
        let buf = [0xC0, 0x7E];
        let res = RstFrame::parse(&buf);

        assert!(res.is_err())
    }

    #[test]
    fn it_rejects_a_corrupted_frame() {
        let buf = [0xC0, 0x00, 0x00, 0x7E];
        let res = RstFrame::parse(&buf);

        assert!(res.is_err())
    }

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
