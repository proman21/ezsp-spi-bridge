use bytes::Buf;
use nom::{
    combinator::consumed,
    error::{Error, ErrorKind},
    sequence::{preceded, tuple},
    Err,
};

use crate::ash::{checksum::crc_digester, types::FrameNumber};

use super::utils::{frame_data_and_flag, FrameFormat, ParserResult};

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
    use nom::bits::{
        bits,
        streaming::{bool, tag, take},
    };
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
        let mut crc = crc_digester();
        let (i2, (ctrl, (res, n_rdy, ack_num))) = consumed(nak_control_byte)(input)?;
        crc.update(ctrl);

        let (rest, mut buf) = frame_data_and_flag(i2)?;
        if buf.len() != 2 {
            return Err(Err::Failure(Error::new(rest, ErrorKind::Complete)));
        }

        let checksum = buf.get_u16();
        if checksum != crc.finalize() {
            return Err(Err::Failure(Error::new(rest, ErrorKind::Verify)));
        }

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
    fn it_parses_valid_frames_correctly() {
        let buf = [0xA6, 0x34, 0xDC, 0x7E];
        let (_rest, frame) = NakFrame::parse(&buf).unwrap();

        assert!(frame.is_ready());
        assert_eq!(*frame.acknowledgement_number(), 6);

        let buf = [0xAD, 0x85, 0xB7, 0x7E];
        let (_rest, frame) = NakFrame::parse(&buf).unwrap();

        assert!(!frame.is_ready());
        assert_eq!(*frame.acknowledgement_number(), 5);
    }

    #[test]
    fn it_fails_to_parse_an_invalid_frame() {
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
    fn it_returns_the_correct_data_field_len() {
        let frame = NakFrame::new(false, true, FrameNumber::new_truncate(6));

        assert_eq!(frame.data_len(), 0);
    }

    #[test]
    fn it_serializes_the_data_field_correctly() {
        let frame = NakFrame::new(false, true, FrameNumber::new_truncate(6));
        let mut buf = BytesMut::zeroed(2);

        frame.serialize_data(&mut buf);
        assert_eq!(*buf, [0; 2])
    }
}
