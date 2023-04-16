use nom::{
    bits::{
        bits,
        complete::{bool, tag, take},
    },
    error::Error,
    sequence::{preceded, tuple},
};

use crate::ash::{buffer::Buffer, types::FrameNumber};

use super::{FrameFormat, ParserResult};

#[derive(Debug)]
pub struct AckFrame {
    res: bool,
    n_rdy: bool,
    ack_num: FrameNumber,
}

impl AckFrame {
    pub fn new(res: bool, n_rdy: bool, ack_num: FrameNumber) -> AckFrame {
        AckFrame {
            res,
            n_rdy,
            ack_num,
        }
    }

    pub fn is_ready(&self) -> bool {
        !self.n_rdy
    }

    pub fn acknowledgement_number(&self) -> FrameNumber {
        self.ack_num
    }
}

fn ack_control_byte(input: Buffer) -> ParserResult<(bool, bool, u8)> {
    bits::<_, _, Error<(Buffer, usize)>, _, _>(preceded(
        tag(0b100, 3usize),
        tuple((bool, bool, take(3usize))),
    ))(input)
}

impl FrameFormat for AckFrame {
    fn flag(&self) -> u8 {
        0x80 | ((self.res as u8) << 4) | ((self.n_rdy as u8) << 3) | *self.ack_num
    }

    fn parse(input: Buffer) -> ParserResult<Self> {
        let (rest, (res, n_rdy, ack_num)) = ack_control_byte(input)?;
        let frame = AckFrame {
            res,
            n_rdy,
            ack_num: FrameNumber::new_truncate(ack_num),
        };
        Ok((rest, frame))
    }
}

#[cfg(test)]
mod tests {
    use bytes::BytesMut;

    use crate::ash::{buffer::Buffer, frame::FrameFormat, types::FrameNumber};

    use super::AckFrame;

    #[test]
    fn it_parses_a_valid_frame_correctly_1() {
        let buf = Buffer::from([0x81].as_ref());
        let (_rest, frame) = AckFrame::parse(buf).unwrap();

        assert!(frame.is_ready());
        assert_eq!(*frame.acknowledgement_number(), 1);
    }

    #[test]
    fn it_parses_a_valid_frame_correctly_2() {
        let buf = Buffer::from([0x8E].as_ref());
        let (_rest, frame) = AckFrame::parse(buf).unwrap();

        assert!(!frame.is_ready());
        assert_eq!(*frame.acknowledgement_number(), 6);
    }

    #[test]
    fn it_fails_to_parse_invalid_frame() {
        let buf = Buffer::from([0x25, 0x42, 0x21, 0xA8, 0x56].as_ref());
        let res = AckFrame::parse(buf);

        assert!(res.is_err());
    }

    #[test]
    fn it_serializes_the_control_byte_correctly() {
        let frame = AckFrame::new(false, true, FrameNumber::new_truncate(6));

        assert_eq!(frame.flag(), 0x8E)
    }

    #[test]
    fn it_returns_correct_data_field_len() {
        let frame = AckFrame::new(false, true, FrameNumber::new_truncate(6));

        assert_eq!(frame.data_len(), 0);
    }

    #[test]
    fn it_serializes_data_field_correctly() {
        let frame = AckFrame::new(false, true, FrameNumber::new_truncate(6));
        let mut buf = BytesMut::zeroed(2);

        frame.serialize_data(&mut buf);
        assert_eq!(*buf, [0; 2])
    }
}
