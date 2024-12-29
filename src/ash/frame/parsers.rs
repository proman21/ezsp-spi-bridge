use super::Frame;
use crate::ash::{
    constants::{ESCAPE_BYTE, FLAG_BYTE},
    Error as AshError, FrameNumber,
};
use bytes::{BufMut, BytesMut};
use nom::{
    bytes::streaming::tag,
    combinator::map_opt,
    error::Error,
    sequence::{preceded, tuple},
    IResult, Needed,
};

type ParserResult<'a, T> = IResult<&'a [u8], T>;

#[derive(Debug)]
pub struct ParseError<'a> {
    pub input: &'a [u8],
    pub error: AshError,
}

impl<'a> ParseError<'a> {
    pub fn new(input: &'a [u8], error: AshError) -> ParseError<'a> {
        ParseError { input, error }
    }

    pub fn into_inner(self) -> (&'a [u8], AshError) {
        (self.input, self.error)
    }
}

pub fn data_control_byte(input: &[u8]) -> ParserResult<Frame> {
    use nom::bits::bits;
    use nom::bits::streaming::{bool, tag, take};

    let (rest, (frm_num, re_tx, ack_num)) = bits::<_, _, Error<(&[u8], usize)>, _, _>(preceded(
        tag(0, 1usize),
        tuple((
            map_opt(take(3usize), FrameNumber::new),
            bool,
            map_opt(take(3usize), FrameNumber::new),
        )),
    ))(input)?;
    Ok((
        rest,
        Frame::Data {
            frm_num,
            re_tx,
            ack_num,
            body: BytesMut::new(),
        },
    ))
}

fn ack_nak_control_byte(pattern: u8) -> impl Fn(&[u8]) -> ParserResult<(bool, bool, FrameNumber)> {
    use nom::bits::{
        bits,
        streaming::{bool, tag, take},
    };
    move |input: &[u8]| {
        bits::<_, _, Error<(&[u8], usize)>, _, _>(preceded(
            tag(pattern, 3usize),
            tuple((bool, bool, map_opt(take(3usize), FrameNumber::new))),
        ))(input)
    }
}

pub fn ack_control_byte(input: &[u8]) -> ParserResult<Frame> {
    let (rest, (res, n_rdy, ack_num)) = ack_nak_control_byte(0b100)(input)?;
    Ok((
        rest,
        Frame::Ack {
            res,
            n_rdy,
            ack_num,
        },
    ))
}

pub fn nak_control_byte(input: &[u8]) -> ParserResult<Frame> {
    let (rest, (res, n_rdy, ack_num)) = ack_nak_control_byte(0b101)(input)?;
    Ok((
        rest,
        Frame::Nak {
            res,
            n_rdy,
            ack_num,
        },
    ))
}

pub fn rst_control_byte(input: &[u8]) -> ParserResult<Frame> {
    let (rest, _) = tag([0xC0])(input)?;
    Ok((rest, Frame::Rst))
}

pub fn rst_ack_control_byte(input: &[u8]) -> ParserResult<Frame> {
    let (rest, _) = tag([0xC1])(input)?;
    Ok((
        rest,
        Frame::RstAck {
            version: 0,
            code: 0,
        },
    ))
}

pub fn error_control_byte(input: &[u8]) -> ParserResult<Frame> {
    let (rest, _) = tag([0xC2])(input)?;
    Ok((
        rest,
        Frame::Error {
            version: 0,
            code: 0,
        },
    ))
}

/// Parses bytes until an unescaped Flag byte is reached, consuming the flag
/// byte. Parser will unescape bytes that are preceded by an Escape byte.
pub fn frame_data_and_flag(input: &[u8]) -> Result<(&[u8], BytesMut), Needed> {
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
            return Err(Needed::new(1));
        }
    }
    Err(Needed::new(1))
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
        let res = frame_data_and_flag(&buf);

        assert!(matches!(res, Err(Needed::Size(size)) if size.get() == 1));
    }

    #[test]
    fn it_requests_more_data_when_escape_byte_is_last_byte() {
        let buf = [0x7D];
        let res = frame_data_and_flag(&buf);

        assert!(matches!(res, Err(Needed::Size(size)) if size.get() == 1));
    }

    #[test]
    fn it_removes_the_flag_byte_from_the_end_of_a_buffer() {
        let buf = [0x01, 0x02, 0x03, 0x7E, 0x04];
        let (rest, res) = frame_data_and_flag(&buf).unwrap();

        assert_eq!(rest, [0x04]);
        assert_eq!(&res[..], [0x01, 0x02, 0x03]);
    }
}
