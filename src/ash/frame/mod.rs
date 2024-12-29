mod parsers;
#[cfg(test)]
mod tests;

pub use parsers::ParseError;

use self::parsers::{
    ack_control_byte, data_control_byte, error_control_byte, frame_data_and_flag, nak_control_byte,
    rst_ack_control_byte, rst_control_byte,
};
use super::{
    checksum::{crc_digester, frame_checksum},
    constants::{ESCAPE_BYTE, FLAG_BYTE, RESERVED_BYTES},
    error::Error as AshError,
    FrameNumber,
};
use bytes::{Buf, BufMut, BytesMut};
use nom::{branch::alt, combinator::consumed, Err, IResult, Needed};
use std::{fmt::Display, iter::successors};

#[derive(Debug, Clone)]
pub enum Frame {
    Data {
        frm_num: FrameNumber,
        re_tx: bool,
        ack_num: FrameNumber,
        body: BytesMut,
    },
    Ack {
        res: bool,
        n_rdy: bool,
        ack_num: FrameNumber,
    },
    Nak {
        res: bool,
        n_rdy: bool,
        ack_num: FrameNumber,
    },
    Rst,
    RstAck {
        version: u8,
        code: u8,
    },
    Error {
        version: u8,
        code: u8,
    },
}

impl Display for Frame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Frame::Data {
                frm_num,
                re_tx,
                ack_num,
                ..
            } => {
                let r = if *re_tx { 0 } else { 1 };
                f.write_fmt(format_args!("DATA({}, {}, {})", **frm_num, **ack_num, r))
            }
            Frame::Ack { n_rdy, ack_num, .. } => {
                let ready = if !n_rdy { "+" } else { "-" };
                f.write_fmt(format_args!("ACK({}){}", **ack_num, ready))
            }
            Frame::Nak { n_rdy, ack_num, .. } => {
                let ready = if !n_rdy { "+" } else { "-" };
                f.write_fmt(format_args!("NAK({}){}", **ack_num, ready))
            }
            Frame::Rst => f.write_str("RST()"),
            Frame::RstAck { version, code } => {
                f.write_fmt(format_args!("RSTACK({}, {})", version, code))
            }
            Frame::Error { version, code } => {
                f.write_fmt(format_args!("ERROR({}, {})", version, code))
            }
        }
    }
}

impl Frame {
    pub fn data(frm_num: FrameNumber, re_tx: bool, ack_num: FrameNumber, body: BytesMut) -> Frame {
        Frame::Data {
            frm_num,
            re_tx,
            ack_num,
            body,
        }
    }

    pub fn ack(n_rdy: bool, ack_num: FrameNumber) -> Frame {
        Frame::Ack {
            res: false,
            n_rdy,
            ack_num,
        }
    }

    pub fn nak(n_rdy: bool, ack_num: FrameNumber) -> Frame {
        Frame::Nak {
            res: false,
            n_rdy,
            ack_num,
        }
    }

    pub fn rst_ack(version: u8, code: u8) -> Frame {
        Frame::RstAck { version, code }
    }

    pub fn error(version: u8, code: u8) -> Frame {
        Frame::Error { version, code }
    }

    /// Try to parse a frame from the given buffer
    pub fn parse(input: &[u8]) -> IResult<&[u8], Frame, ParseError> {
        let mut crc = crc_digester();
        let control_byte_res = consumed(alt((
            data_control_byte,
            ack_control_byte,
            nak_control_byte,
            rst_control_byte,
            rst_ack_control_byte,
            error_control_byte,
        )))(&input[..]);
        let (i2, (ctrl, mut frame)) = match control_byte_res {
            Ok(v) => v,
            Err(_) => {
                let (rest, _) = frame_data_and_flag(input).map_err(Err::Incomplete)?;
                return Err(Err::Failure(ParseError::new(rest, AshError::UnknownFrame)));
            }
        };
        crc.update(ctrl);

        let (rest, mut data_and_checksum) = frame_data_and_flag(i2).map_err(Err::Incomplete)?;

        let mut checksum_bytes: BytesMut;
        if let Needed::Size(s) = frame.data_len() {
            let size = s.get();
            if data_and_checksum.len() != size {
                return Err(Err::Failure(ParseError::new(
                    rest,
                    AshError::InvalidDataField(frame),
                )));
            }
            checksum_bytes = data_and_checksum.split_off(size - 2);
        } else {
            if data_and_checksum.len() < 2 {
                return Err(Err::Failure(ParseError::new(
                    rest,
                    AshError::InvalidDataField(frame),
                )));
            }
            checksum_bytes = data_and_checksum.split_off(data_and_checksum.len() - 2);
        }
        crc.update(&data_and_checksum);
        let checksum = checksum_bytes.get_u16();
        if crc.finalize() != checksum {
            return Err(Err::Failure(ParseError::new(
                rest,
                AshError::InvalidChecksum(frame),
            )));
        }

        match frame {
            Frame::Data { ref mut body, .. } => {
                *body = data_and_checksum;
            }
            Frame::RstAck {
                ref mut version,
                ref mut code,
            }
            | Frame::Error {
                ref mut version,
                ref mut code,
            } => {
                *version = data_and_checksum.get_u8();
                *code = data_and_checksum.get_u8();
            }
            _ => {}
        }

        Ok((rest, frame))
    }

    /// Serialize the frame and write it into a buffer
    pub fn serialize(&self, buf: &mut BytesMut) {
        buf.put_u8(self.flag());
        self.serialize_data(buf);

        let checksum = frame_checksum(buf);
        for mut byte in checksum.to_be_bytes() {
            if RESERVED_BYTES.contains(&byte) {
                byte ^= 0x20;
            }
            buf.put_u8(byte);
        }
        buf.put_u8(FLAG_BYTE);
    }

    fn flag(&self) -> u8 {
        match &self {
            Frame::Data {
                frm_num,
                re_tx,
                ack_num,
                ..
            } => (**frm_num << 4) | ((*re_tx as u8) << 3) | **ack_num,
            Frame::Ack {
                res,
                n_rdy,
                ack_num,
            } => 0x80 | ((*res as u8) << 4) | ((*n_rdy as u8) << 3) | **ack_num,
            Frame::Nak {
                res,
                n_rdy,
                ack_num,
            } => 0xA0 | ((*res as u8) << 4) | ((*n_rdy as u8) << 3) | **ack_num,
            Frame::Rst => 0xC0,
            Frame::RstAck { .. } => 0xC1,
            Frame::Error { .. } => 0xC2,
        }
    }

    /// The amount of data expected in the frame body and the two checksum bytes
    fn data_len(&self) -> Needed {
        match self {
            Frame::Data { .. } => Needed::Unknown,
            Frame::RstAck { .. } | Frame::Error { .. } => Needed::new(4),
            _ => Needed::new(2),
        }
    }

    fn serialize_data(&self, buf: &mut BytesMut) {
        match self {
            Frame::Data { body, .. } => {
                buf.reserve(body.len());

                for (byte, seq) in body.iter().zip(rand_seq()) {
                    let mut res = byte ^ seq;
                    if RESERVED_BYTES.contains(&res) {
                        res ^= 0x20;
                        buf.put_u8(ESCAPE_BYTE);
                    }
                    buf.put_u8(res);
                }
            }
            Frame::RstAck { version, code } => {
                buf.reserve(2);
                buf.put_u8(*version);
                buf.put_u8(*code);
            }
            Frame::Error { version, code } => {
                buf.reserve(2);
                buf.put_u8(*version);
                buf.put_u8(*code);
            }
            _ => {}
        }
    }
}

fn rand_seq() -> impl Iterator<Item = u8> {
    successors(Some(0x42), |b| Some((b >> 1) ^ ((b & 0x01) * 0xB8)))
}
