use nom::{
    bits::{
        bits,
        complete::{bool, tag, take},
    },
    error::Error,
    sequence::{preceded, tuple},
};

use crate::ash::types::FrameNumber;

use super::{FrameFormat, ParserResult};

#[derive(Debug)]
pub struct NakFrame {
    res: bool,
    n_rdy: bool,
    ack_num: FrameNumber,
}

impl NakFrame {
    pub fn new(res: bool, n_rdy: bool, ack_num: FrameNumber) -> NakFrame {
        NakFrame {
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

fn nak_control_byte(input: &[u8]) -> ParserResult<(bool, bool, u8)> {
    bits::<_, _, Error<(&[u8], usize)>, _, _>(preceded(
        tag(0b101, 3usize),
        tuple((bool, bool, take(3usize))),
    ))(input)
}

impl FrameFormat for NakFrame {
    fn flag(&self) -> u8 {
        0xA0 | ((self.res as u8) << 4) | ((self.n_rdy as u8) << 3) | *self.ack_num
    }

    fn parse(input: &[u8]) -> ParserResult<Self> {
        let (rest, (res, n_rdy, ack_num)) = nak_control_byte(input)?;
        let frame = NakFrame {
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

    use crate::ash::{frame::FrameFormat, types::FrameNumber};

    use super::NakFrame;

    #[test]
    fn it_parses_a_valid_frame_correctly_1() {
        let buf = [0xA6];
        let (_rest, frame) = NakFrame::parse(&buf).unwrap();

        assert!(frame.is_ready());
        assert_eq!(*frame.acknowledgement_number(), 6);
    }

    #[test]
    fn it_parses_a_valid_frame_correctly_2() {
        let buf = [0xAD];
        let (_rest, frame) = NakFrame::parse(&buf).unwrap();

        assert!(!frame.is_ready());
        assert_eq!(*frame.acknowledgement_number(), 5);
    }

    #[test]
    fn it_fails_to_parse_invalid_frame() {
        let buf = [0x25, 0x42, 0x21, 0xA8, 0x56];
        let res = NakFrame::parse(&buf);

        assert!(res.is_err());
    }

    #[test]
    fn it_serializes_the_control_byte_correctly() {
        let frame = NakFrame::new(false, true, FrameNumber::new_truncate(5));

        assert_eq!(frame.flag(), 0xAD)
    }

    #[test]
    fn it_returns_correct_data_field_len() {
        let frame = NakFrame::new(false, true, FrameNumber::new_truncate(6));

        assert_eq!(frame.data_len(), 0);
    }

    #[test]
    fn it_serializes_data_field_correctly() {
        let frame = NakFrame::new(false, true, FrameNumber::new_truncate(6));
        let mut buf = BytesMut::zeroed(2);

        frame.serialize_data(&mut buf);
        assert_eq!(*buf, [0; 2])
    }
}
