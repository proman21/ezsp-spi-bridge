use std::iter::{successors, zip};

use bytes::{Buf, BufMut, Bytes, BytesMut};

use nom::{
    combinator::consumed,
    error::{Error, ErrorKind},
    sequence::{preceded, tuple},
};
use nom::{Err, IResult};

use crate::ash::types::FrameNumber;
use crate::ash::{
    checksum::crc_digester,
    constants::{ESCAPE_BYTE, RESERVED_BYTES},
};

use super::utils::{frame_data_and_flag, FrameFormat, ParserResult};

fn rand_seq() -> impl Iterator<Item = u8> {
    successors(Some(0x42), |b| Some((b >> 1) ^ ((b & 0x01) * 0xB8)))
}

fn xor_with_rand_seq(buf: &mut [u8]) {
    for (byte, seq) in zip(buf, rand_seq()) {
        *byte ^= seq;
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

fn data_control_byte(input: &[u8]) -> IResult<&[u8], (u8, bool, u8)> {
    use nom::bits::bits;
    use nom::bits::streaming::{bool, tag, take};

    bits::<_, _, Error<(&[u8], usize)>, _, _>(preceded(
        tag(0, 1usize),
        tuple((take(3usize), bool, take(3usize))),
    ))(input)
}

impl FrameFormat for DataFrame {
    fn serialize_data(&self, buf: &mut BytesMut) {
        buf.reserve(self.data_len());

        for (byte, seq) in self.data.iter().zip(rand_seq()) {
            let mut res = byte ^ seq;
            if RESERVED_BYTES.contains(&res) {
                res ^= 0x20;
                buf.put_u8(ESCAPE_BYTE);
            }
            buf.put_u8(res);
        }
    }

    fn data_len(&self) -> usize {
        self.data.len()
    }

    fn flag(&self) -> u8 {
        (*self.frm_num << 4) | ((self.re_tx as u8) << 3) | *self.ack_num
    }

    fn parse(input: &[u8]) -> ParserResult<Self> {
        let mut crc = crc_digester();

        let (i2, (ctrl, (frm_num, re_tx, ack_num))) = consumed(data_control_byte)(input)?;
        crc.update(ctrl);

        let (rest, mut data) = frame_data_and_flag(i2)?;

        if data.len() < 2 {
            return Err(Err::Failure(Error::new(rest, ErrorKind::Complete)));
        }

        let mut checksum_buf = data.split_off(data.len() - 2);
        crc.update(&data);
        let checksum = checksum_buf.get_u16();
        if crc.finalize() != checksum {
            return Err(Err::Failure(Error::new(rest, ErrorKind::Verify)));
        }

        xor_with_rand_seq(&mut data);

        let frame = DataFrame {
            frm_num: FrameNumber::new_truncate(frm_num),
            re_tx,
            ack_num: FrameNumber::new_truncate(ack_num),
            data: data.freeze(),
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
        let buf = [0x25, 0x42, 0x21, 0xA8, 0x56, 0xA6, 0x09, 0x7E];
        let (_rest, frame) = DataFrame::parse(&buf).unwrap();

        assert_eq!(*frame.frame_number(), 2);
        assert!(!frame.is_retransmitted());
        assert_eq!(*frame.acknowledgement_number(), 5);
        assert_eq!(frame.data().as_ref(), [0x00, 0x00, 0x00, 0x02])
    }

    #[test]
    fn it_fails_to_parse_invalid_frame() {
        let buf = [0xA6];
        let res = DataFrame::parse(&buf);
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
        let mut buf = BytesMut::with_capacity(4);
        frame.serialize_data(&mut buf);
        assert_eq!(*buf, [0x42, 0x21, 0xA8, 0x56]);
    }
}
