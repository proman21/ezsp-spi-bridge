mod ack;
mod data;
mod error;
mod nak;
mod rst;
mod rst_ack;
mod utils;

use bytes::{BufMut, BytesMut};
use nom::{
    branch::alt,
    combinator::{cut, map},
};

use super::{
    checksum::frame_checksum,
    constants::{FLAG_BYTE, RESERVED_BYTES},
};

use self::utils::{FrameFormat, ParserResult};
pub use self::{
    ack::AckFrame, data::DataFrame, error::ErrorFrame, nak::NakFrame, rst::RstFrame,
    rst_ack::RstAckFrame,
};

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
    /// Try to parse a frame from the given buffer
    pub fn parse(buf: &[u8]) -> ParserResult<Frame> {
        FrameFormat::parse(buf)
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
}

impl FrameFormat for Frame {
    fn parse(input: &[u8]) -> ParserResult<Self> {
        // Parser needs to handle escaped bytes correctly
        cut(alt((
            map(DataFrame::parse, Frame::Data),
            map(AckFrame::parse, Frame::Ack),
            map(NakFrame::parse, Frame::Nak),
            map(RstFrame::parse, Frame::Rst),
            map(RstAckFrame::parse, Frame::RstAck),
            map(ErrorFrame::parse, Frame::Error),
        )))(&input[..])
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

    fn serialize_data(&self, buf: &mut BytesMut) {
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
    use super::Frame;
    use nom::Err;

    #[test]
    fn it_rejects_an_unknown_frame_type() {
        let buf = [0xFF];
        let res = Frame::parse(&buf).unwrap_err();

        assert!(matches!(res, Err::Failure(_)));
    }

    #[test]
    fn it_parses_a_valid_data_frame() {
        let buf = [0x25, 0x00, 0x00, 0x00, 0x02, 0x1A, 0xAD, 0x7E];
        let (rest, frame) = Frame::parse(&buf).unwrap();

        assert_eq!(rest.len(), 0);
        assert!(matches!(frame, Frame::Data(_)))
    }

    #[test]
    fn it_parses_a_valid_ack_frame() {
        let buf = [0x81, 0x60, 0x59, 0x7E];
        let (rest, frame) = Frame::parse(&buf).unwrap();

        assert_eq!(rest.len(), 0);
        assert!(matches!(frame, Frame::Ack(_)))
    }

    #[test]
    fn it_parses_a_valid_error_frame() {
        let buf = [0xC2, 0x02, 0x51, 0xA8, 0xBD, 0x7E];
        let (rest, frame) = Frame::parse(&buf).unwrap();

        assert_eq!(rest.len(), 0);
        assert!(matches!(frame, Frame::Error(_)))
    }

    #[test]
    fn it_parses_a_valid_nak_frame() {
        let buf = [0xA6, 0x34, 0xDC, 0x7E];
        let (rest, frame) = Frame::parse(&buf).unwrap();

        assert_eq!(rest.len(), 0);
        assert!(matches!(frame, Frame::Nak(_)))
    }

    #[test]
    fn it_parses_a_valid_rst_ack_frame() {
        let buf = [0xC1, 0x02, 0x02, 0x9B, 0x7B, 0x7E];
        let (rest, frame) = Frame::parse(&buf).unwrap();

        assert_eq!(rest.len(), 0);
        assert!(matches!(frame, Frame::RstAck(_)))
    }

    #[test]
    fn it_parses_a_valid_rst_frame() {
        let buf = [0xC0, 0x38, 0xBC, 0x7E];
        let (rest, frame) = Frame::parse(&buf).unwrap();

        assert_eq!(rest.len(), 0);
        assert!(matches!(frame, Frame::Rst(_)))
    }
}
