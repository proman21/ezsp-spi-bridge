use bytes::Bytes;
use nom::sequence::Tuple;
use nom::{
    bits::{
        bits,
        complete::{bool, tag, take},
    },
    combinator::{rest, verify},
    error::Error,
    sequence::{preceded, tuple},
};

use crate::ash::types::FrameNumber;
use crate::buffer::BufferMut;

use super::{FrameFormat, ParserResult};

fn randomize_data(buf: &mut [u8]) {
    let mut reg: u8 = 0x42;
    for item in buf {
        *item ^= reg;
        reg = (reg >> 1) ^ ((reg & 0x01) * 0xB8)
    }
}

#[derive(Debug)]
pub struct DataFrame {
    frm_num: FrameNumber,
    re_tx: bool,
    ack_num: FrameNumber,
    data: Bytes,
}

impl DataFrame {
    pub fn new(frm_num: FrameNumber, re_tx: bool, ack_num: FrameNumber, data: Bytes) -> DataFrame {
        DataFrame {
            frm_num,
            re_tx,
            ack_num,
            data,
        }
    }

    pub fn frame_number(&self) -> FrameNumber {
        self.frm_num
    }

    pub fn is_retransmitted(&self) -> bool {
        self.re_tx
    }

    pub fn acknowledgement_number(&self) -> FrameNumber {
        self.ack_num
    }

    pub fn data(&self) -> &Bytes {
        &self.data
    }
}

fn data_control_byte(input: BufferMut) -> ParserResult<(u8, bool, u8)> {
    bits::<_, _, Error<(BufferMut, usize)>, _, _>(preceded(
        tag(0, 1usize),
        tuple((take(3usize), bool, take(3usize))),
    ))(input)
}

impl FrameFormat for DataFrame {
    fn serialize_data(&self, buf: &mut [u8]) {
        buf[..self.data.len()].copy_from_slice(&self.data);
        randomize_data(&mut buf[..self.data.len()]);
    }

    fn data_len(&self) -> usize {
        self.data.len()
    }

    fn flag(&self) -> u8 {
        (*self.frm_num << 4) | ((self.re_tx as u8) << 3) | *self.ack_num
    }

    fn parse(input: BufferMut) -> ParserResult<Self> {
        let (rest, ((frm_num, re_tx, ack_num), mut xor_data)) = (
            data_control_byte,
            verify(rest, |d: &[u8]| (d.len() >= 3) && (d.len() <= 128)),
        )
            .parse(input)?;

        randomize_data(&mut xor_data);

        let frame = DataFrame {
            frm_num: FrameNumber::new_truncate(frm_num),
            re_tx,
            ack_num: FrameNumber::new_truncate(ack_num),
            data: xor_data.freeze(),
        };
        Ok((rest, frame))
    }
}

#[cfg(test)]
mod tests {
    use bytes::BytesMut;

    use super::*;

    #[test]
    fn it_parses_a_valid_frame_correctly() {
        let buf = BufferMut::from([0x25, 0x42, 0x21, 0xA8, 0x56].as_ref());
        let (_rest, frame) = DataFrame::parse(buf).unwrap();

        assert_eq!(*frame.frame_number(), 2);
        assert!(!frame.is_retransmitted());
        assert_eq!(*frame.acknowledgement_number(), 5);
        assert_eq!(frame.data().as_ref(), [0x00, 0x00, 0x00, 0x02])
    }

    #[test]
    fn it_fails_to_parse_invalid_frame() {
        let buf = BufferMut::from([0xA6].as_ref());
        let res = DataFrame::parse(buf);
        assert!(res.is_err());
    }

    #[test]
    fn it_serializes_the_control_byte_correctly() {
        let frame = DataFrame::new(
            FrameNumber::new_truncate(2),
            false,
            FrameNumber::new_truncate(5),
            Bytes::new(),
        );
        assert_eq!(frame.flag(), 0x25);
    }

    #[test]
    fn it_returns_correct_data_field_len() {
        let frame = DataFrame::new(
            FrameNumber::new_truncate(2),
            false,
            FrameNumber::new_truncate(5),
            Bytes::from_static(&[0x00, 0x00, 0x00, 0x02]),
        );
        assert_eq!(frame.data_len(), 4);
    }

    #[test]
    fn it_serializes_data_field_correctly() {
        let frame = DataFrame::new(
            FrameNumber::new_truncate(2),
            false,
            FrameNumber::new_truncate(5),
            Bytes::from_static(&[0x00, 0x00, 0x00, 0x02]),
        );
        let mut buf = BytesMut::zeroed(4);
        frame.serialize_data(&mut buf);
        assert_eq!(*buf, [0x42, 0x21, 0xA8, 0x56]);
    }
}
