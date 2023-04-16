mod ack;
mod data;
mod error;
mod nak;
mod rst;
mod rst_ack;

use std::io::Cursor;

use bytes::{Buf, BufMut, BytesMut};
use nom::{
    branch::alt,
    combinator::{all_consuming, map},
    Finish, IResult,
};

use super::{checksum::frame_checksum, escaping::escape_reserved_bytes};
use crate::ash::escaping::unescape_reserved_bytes;

pub use self::{
    ack::AckFrame, data::DataFrame, error::ErrorFrame, nak::NakFrame, rst::RstFrame,
    rst_ack::RstAckFrame,
};

use super::error::{Error, Result};

pub const FLAG_BYTE: u8 = 0x7E;
pub const SUB_BYTE: u8 = 0x18;
pub const CANCEL_BYTE: u8 = 0x1A;
pub const ESCAPE_BYTE: u8 = 0x7D;

pub type ParserResult<'a, O> = IResult<&'a [u8], O>;

pub trait FrameFormat: Sized {
    fn parse(input: &[u8]) -> ParserResult<Self>;
    fn flag(&self) -> u8;
    fn data_len(&self) -> usize {
        0
    }
    fn serialize_data(&self, _buf: &mut [u8]) {}
}

#[derive(Debug)]
pub enum Frame {
    Data(DataFrame),
    Ack(AckFrame),
    Nak(NakFrame),
    Rst(RstFrame),
    RstAck(RstAckFrame),
    Error(ErrorFrame),
}

impl Frame {
    /// Check if a full frame can be found in the buffer, and if the frame
    /// checksum is valid
    pub fn check(buf: &mut Cursor<&mut [u8]>) -> Result<()> {
        // Search for a Flag byte
        let len = buf
            .get_ref()
            .iter()
            .position(|&b| b == FLAG_BYTE)
            .ok_or(Error::Incomplete)?;

        // Extract the preceding bytes from the buffer
        buf.advance(len + 1);
        if len < 2 {
            return Err(Error::UnknownFrame);
        }

        let frame = &mut buf.get_mut()[..len];

        // Unstuff the reserved bytes
        unescape_reserved_bytes(frame);

        // Extract the checksum from the frame
        let checksum = (&frame[len - 2..]).get_u16();

        // Calculate the checksum and validate
        let crc = frame_checksum(&frame[..len - 2]);
        if crc != checksum {
            return Err(Error::InvalidChecksum);
        }

        Ok(())
    }

    /// Try to parse a frame from the given buffer
    pub fn parse(buf: &[u8]) -> Result<Frame> {
        FrameFormat::parse(buf)
            .finish()
            .map(|(_, frame)| frame)
            .map_err(Error::from)
    }

    /// Serialize the frame and write it into a buffer
    pub fn serialize(&self, buf: &mut BytesMut) {
        buf.put_u8(self.flag());
        self.serialize_data(buf);
        let checksum = frame_checksum(buf);
        buf.put_u16(checksum);
        let unescaped_bytes = buf.split().freeze();
        escape_reserved_bytes(&unescaped_bytes, buf);
        buf.put_u8(FLAG_BYTE);
    }
}

impl FrameFormat for Frame {
    fn parse(input: &[u8]) -> ParserResult<Self> {
        all_consuming(alt((
            map(DataFrame::parse, Frame::Data),
            map(AckFrame::parse, Frame::Ack),
            map(NakFrame::parse, Frame::Nak),
            map(RstFrame::parse, Frame::Rst),
            map(RstAckFrame::parse, Frame::RstAck),
            map(ErrorFrame::parse, Frame::Error),
        )))(input)
    }

    fn data_len(&self) -> usize {
        match &self {
            Frame::Data(f) => f.data_len(),
            Frame::Ack(f) => f.data_len(),
            Frame::Nak(f) => f.data_len(),
            Frame::Rst(f) => f.data_len(),
            Frame::RstAck(f) => f.data_len(),
            Frame::Error(f) => f.data_len(),
        }
    }

    fn flag(&self) -> u8 {
        match &self {
            Frame::Data(f) => f.flag(),
            Frame::Ack(f) => f.flag(),
            Frame::Nak(f) => f.flag(),
            Frame::Rst(f) => f.flag(),
            Frame::RstAck(f) => f.flag(),
            Frame::Error(f) => f.flag(),
        }
    }

    fn serialize_data(&self, buf: &mut [u8]) {
        match &self {
            Frame::Data(f) => f.serialize_data(buf),
            Frame::Ack(f) => f.serialize_data(buf),
            Frame::Nak(f) => f.serialize_data(buf),
            Frame::Rst(f) => f.serialize_data(buf),
            Frame::RstAck(f) => f.serialize_data(buf),
            Frame::Error(f) => f.serialize_data(buf),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::{Frame, FLAG_BYTE};

    use crate::ash::error::Error;

    #[test]
    fn check_fails_when_no_flag_byte_is_found() {
        let mut buf = [0u8; 16];
        let mut cursor = Cursor::new(&mut buf[..]);

        let err = Frame::check(&mut cursor).unwrap_err();
        assert_eq!(err, Error::Incomplete);
        assert_eq!(cursor.position(), 0);
    }

    #[test]
    fn check_fails_when_frame_is_too_short() {
        let mut buf = [FLAG_BYTE, 0, 0];
        let mut cursor = Cursor::new(&mut buf[..]);

        let err = Frame::check(&mut cursor).unwrap_err();
        assert_eq!(err, Error::UnknownFrame);
        assert_eq!(cursor.position(), 1);
    }

    #[test]
    fn check_fails_when_the_checksum_is_invalid() {
        let mut buf = [0x25, 0x00, 0x00, 0x00, 0x02, 0x1A, 0xAF, 0x7E];
        let mut cursor = Cursor::new(&mut buf[..]);

        let err = Frame::check(&mut cursor).unwrap_err();
        assert_eq!(err, Error::InvalidChecksum);
        assert_eq!(cursor.position(), 8);
    }

    #[test]
    fn check_passes_when_valid_frame_found() {
        let mut buf = [0x25, 0x00, 0x00, 0x00, 0x02, 0x1A, 0xAD, 0x7E];
        let mut cursor = Cursor::new(&mut buf[..]);

        assert!(Frame::check(&mut cursor).is_ok());
        assert_eq!(cursor.position(), 8);
    }

    #[test]
    fn parse_fails_when_frame_data_field_is_too_long() {
        let buf = [0x81, 0x42, 0x32, 0xBD, 0x49, 0x7E];
        let err = Frame::parse(&buf).unwrap_err();

        assert_eq!(err, Error::InvalidDataField)
    }

    #[test]
    fn parse_fails_when_an_unknown_frame_is_found() {}

    #[test]
    fn parse_succeds_when_valid_frame_exists() {
        let buf = [0x25, 0x00, 0x00, 0x00, 0x02, 0x1A, 0xAD, 0x7E];
        let res = Frame::parse(&buf);

        assert!(res.is_ok())
    }
}
