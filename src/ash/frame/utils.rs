use bytes::{BufMut, BytesMut};
use nom::{Err, IResult, Needed};

use crate::ash::constants::{ESCAPE_BYTE, FLAG_BYTE};

pub type ParserResult<'a, T> = IResult<&'a [u8], T>;
pub trait FrameFormat: Sized {
    fn parse<'a>(input: &'a [u8]) -> ParserResult<'a, Self>;
    fn flag(&self) -> u8;
    fn data_len(&self) -> usize {
        0
    }
    fn serialize_data(&self, _buf: &mut BytesMut) {}
}

/// Parses bytes until an unescaped Flag byte is reached, consuming the flag
/// byte. Parser will unescape bytes that are preceded by an Escape byte.
pub fn frame_data_and_flag(input: &[u8]) -> ParserResult<BytesMut> {
    let mut collector = BytesMut::new();
    let mut i = 0;

    while let Some(j) = input[i..]
        .iter()
        .position(|&b| b == FLAG_BYTE || b == ESCAPE_BYTE)
    {
        collector.extend_from_slice(&input[i..i + j]);
        i += j;
        if input[i] == FLAG_BYTE {
            return Ok((&input[i + 1..], collector));
        }
        i += 1;
        if input[i..].len() >= 1 {
            collector.put_u8(input[i] ^ 0x20);
            i += 1;
        } else {
            return Err(Err::Incomplete(Needed::new(1)));
        }
    }
    Err(Err::Incomplete(Needed::new(1)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_escapes_reserved_bytes() {
        let buf = [
            0x7D, 0x5E, 0x7D, 0x5D, 0x7D, 0x31, 0x7D, 0x33, 0x7D, 0x38, 0x7D, 0x3A, 0x7E,
        ];
        let (rest, res) = frame_data_and_flag(&buf).unwrap();

        assert_eq!(rest.len(), 0);
        assert_eq!(&res[..], [0x7E, 0x7D, 0x11, 0x13, 0x18, 0x1A]);
    }

    #[test]
    fn it_requests_more_data_when_frame_body_is_empty() {
        let buf = [];
        let err = frame_data_and_flag(&buf).unwrap_err();

        assert!(err.is_incomplete())
    }

    #[test]
    fn it_requests_more_data_when_escape_byte_is_last_byte() {
        let buf = [0x7D];
        let err = frame_data_and_flag(&buf).unwrap_err();

        assert!(err.is_incomplete())
    }

    #[test]
    fn it_removes_the_flag_byte_from_the_end_of_a_buffer() {
        let buf = [0x01, 0x02, 0x03, 0x7E, 0x04];
        let (rest, res) = frame_data_and_flag(&buf).unwrap();

        assert_eq!(rest, [0x04]);
        assert_eq!(&res[..], [0x01, 0x02, 0x03]);
    }
}
