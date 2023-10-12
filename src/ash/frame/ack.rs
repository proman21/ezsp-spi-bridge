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

fn ack_control_byte(input: &[u8]) -> ParserResult<(bool, bool, u8)> {
    use nom::bits::{
        bits,
        streaming::{bool, tag, take},
    };
    bits::<_, _, Error<(&[u8], usize)>, _, _>(preceded(
        tag(0b100, 3usize),
        tuple((bool, bool, take(3usize))),
    ))(input)
}

impl FrameFormat for AckFrame {
    fn flag(&self) -> u8 {
        0x80 | ((self.res as u8) << 4) | ((self.n_rdy as u8) << 3) | *self.ack_num
    }

    fn parse(input: &[u8]) -> ParserResult<Self> {
        let mut crc = crc_digester();

        let (rest, ((ctrl, (res, n_rdy, ack_num)), mut checksum_buf)) =
            tuple((consumed(ack_control_byte), frame_data_and_flag))(input)?;
        crc.update(ctrl);

        if checksum_buf.len() != 2 {
            return Err(Err::Failure(Error::new(rest, ErrorKind::Complete)));
        }
        let checksum = checksum_buf.get_u16();
        if crc.finalize() != checksum {
            return Err(Err::Failure(Error::new(rest, ErrorKind::Verify)));
        };

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
    use super::AckFrame;
    use crate::ash::{frame::FrameFormat, types::FrameNumber};
    use bytes::BytesMut;

    #[test]
    fn it_parses_valid_frames_correctly() {
        let buf = [0x81, 0x60, 0x59, 0x7E];
        let (_rest, frame) = AckFrame::parse(&buf).unwrap();

        assert!(frame.is_ready());
        assert_eq!(*frame.acknowledgement_number(), 1);

        let buf = [0x8E, 0x91, 0xB6, 0x7E];
        let (_rest, frame) = AckFrame::parse(&buf).unwrap();

        assert!(!frame.is_ready());
        assert_eq!(*frame.acknowledgement_number(), 6);
    }

    #[test]
    fn it_rejects_an_early_flag_byte() {
        let buf = [0x8E, 0x7E];
        let res = AckFrame::parse(&buf);

        assert!(res.is_err());
    }

    #[test]
    fn it_rejects_a_non_ack_frame() {
        let buf = [0x25, 0x42, 0x21, 0xA8, 0x56];
        let res = AckFrame::parse(&buf);

        assert!(res.is_err());
    }

    #[test]
    fn it_serializes_the_control_byte_correctly() {
        let frame = AckFrame::new(false, true, FrameNumber::new_truncate(6));

        assert_eq!(frame.flag(), 0x8E)
    }

    #[test]
    fn it_returns_the_correct_data_field_len() {
        let frame = AckFrame::new(false, true, FrameNumber::new_truncate(6));

        assert_eq!(frame.data_len(), 0);
    }

    #[test]
    fn it_serializes_the_data_field_correctly() {
        let frame = AckFrame::new(false, true, FrameNumber::new_truncate(6));
        let mut buf = BytesMut::zeroed(2);

        frame.serialize_data(&mut buf);
        assert_eq!(*buf, [0; 2])
    }
}
