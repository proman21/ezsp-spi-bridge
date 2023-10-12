use bytes::{Buf, Bytes};
use nom::{
    bits::{
        bits,
        streaming::{bool, tag as bits_tag, take as bits_take},
    },
    bytes::streaming::{tag, take},
    combinator::{flat_map, map, value},
    error::Error,
    number::streaming::u8,
    sequence::{preceded, terminated},
    IResult,
};

use crate::buffers::Buffer;

#[derive(Debug, Clone, PartialEq)]
pub enum Response {
    EzspFrame(Bytes),
    BootloaderFrame(Bytes),
    SpiStatus(bool),
    SpiProtocolVersion(u8),
    NcpReset(u8),
    OversizedPayloadFrame,
    AbortedTransaction,
    MissingFrameTerminator,
    UnsupportedSpiCommand,
}

pub type ParserResult<O> = IResult<Buffer, O>;

impl Response {
    pub fn parse(input: Buffer) -> ParserResult<Response> {
        terminated(
            nom::branch::alt((
                Response::parse_ncp_reset,
                Response::parse_oversized_payload_frame,
                Response::parse_aborted_transaction,
                Response::parse_missing_frame_terminator,
                Response::parse_unsupported_spi_command,
                Response::parse_spi_protocol_version,
                Response::parse_spi_status,
                Response::parse_bootloader_frame,
                Response::parse_ezsp_frame,
            )),
            tag([0xA7]),
        )(input)
    }

    fn parse_ncp_reset(input: Buffer) -> ParserResult<Response> {
        preceded(
            tag([0x00]),
            map(take(1usize), |mut i: Buffer| Response::NcpReset(i.get_u8())),
        )(input)
    }

    fn parse_oversized_payload_frame(input: Buffer) -> ParserResult<Response> {
        preceded(
            tag([0x01]),
            value(Response::OversizedPayloadFrame, take(1usize)),
        )(input)
    }

    fn parse_aborted_transaction(input: Buffer) -> ParserResult<Response> {
        preceded(
            tag([0x02]),
            value(Response::AbortedTransaction, take(1usize)),
        )(input)
    }

    fn parse_missing_frame_terminator(input: Buffer) -> ParserResult<Response> {
        preceded(
            tag([0x03]),
            value(Response::MissingFrameTerminator, take(1usize)),
        )(input)
    }

    fn parse_unsupported_spi_command(input: Buffer) -> ParserResult<Response> {
        preceded(
            tag([0x04]),
            value(Response::UnsupportedSpiCommand, take(1usize)),
        )(input)
    }

    fn parse_spi_protocol_version(input: Buffer) -> ParserResult<Response> {
        bits::<_, _, Error<(Buffer, usize)>, _, _>(preceded(
            bits_tag(0b10, 2usize),
            map(bits_take(6usize), Response::SpiProtocolVersion),
        ))(input)
    }

    fn parse_spi_status(input: Buffer) -> ParserResult<Response> {
        bits::<_, _, Error<(Buffer, usize)>, _, _>(preceded(
            bits_tag(0x60, 7usize),
            map(bool, Response::SpiStatus),
        ))(input)
    }

    fn parse_bootloader_frame(input: Buffer) -> ParserResult<Response> {
        preceded(
            tag([0xFD]),
            map(flat_map(u8, take), |b: Buffer| {
                Response::BootloaderFrame(b.into_inner())
            }),
        )(input)
    }

    fn parse_ezsp_frame(input: Buffer) -> ParserResult<Response> {
        preceded(
            tag([0xFE]),
            map(flat_map(u8, take), |b: Buffer| {
                Response::EzspFrame(b.into_inner())
            }),
        )(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_parses_ncp_reset_response() {
        let buf = Buffer::from_static(&[0x00, 0x02, 0xA7]);
        let (_rest, res) = Response::parse(buf).unwrap();

        assert_eq!(res, Response::NcpReset(0x02));
    }

    #[test]
    fn it_parse_oversized_payload_response() {
        let buf = Buffer::from_static(&[0x01, 0x00, 0xA7]);
        let (_rest, res) = Response::parse(buf).unwrap();

        assert_eq!(res, Response::OversizedPayloadFrame);
    }

    #[test]
    fn it_parses_aborted_transaction_response() {
        let buf = Buffer::from_static(&[0x02, 0x00, 0xA7]);
        let (_rest, res) = Response::parse(buf).unwrap();

        assert_eq!(res, Response::AbortedTransaction);
    }

    #[test]
    fn it_parses_missing_frame_terminator_response() {
        let buf = Buffer::from_static(&[0x03, 0x00, 0xA7]);
        let (_rest, res) = Response::parse(buf).unwrap();

        assert_eq!(res, Response::MissingFrameTerminator);
    }

    #[test]
    fn it_parses_unsupported_spi_command_response() {
        let buf = Buffer::from_static(&[0x04, 0x00, 0xA7]);
        let (_rest, res) = Response::parse(buf).unwrap();

        assert_eq!(res, Response::UnsupportedSpiCommand);
    }

    #[test]
    fn it_parses_spi_protocol_version_response() {
        let buf = Buffer::from_static(&[0xAA, 0xA7]);
        let (_rest, res) = Response::parse(buf).unwrap();

        assert_eq!(res, Response::SpiProtocolVersion(0x2A))
    }

    #[test]
    fn it_parses_spi_status_response() {
        let buf = Buffer::from_static(&[0xC1, 0xA7]);
        let (_rest, res) = Response::parse(buf).unwrap();

        assert_eq!(res, Response::SpiStatus(true));
    }

    #[test]
    fn it_parses_bootloader_frame_response() {
        let buf = Buffer::from_static(&[0xFD, 0x03, 0x01, 0x02, 0x03, 0xA7]);
        let (_rest, res) = Response::parse(buf).unwrap();

        assert_eq!(
            res,
            Response::BootloaderFrame(Bytes::from_static(&[0x01, 0x02, 0x03]))
        )
    }

    #[test]
    fn it_parses_ezsp_frame_response() {
        let buf = Buffer::from_static(&[0xFE, 0x03, 0x01, 0x02, 0x03, 0xA7]);
        let (_rest, res) = Response::parse(buf).unwrap();

        assert_eq!(
            res,
            Response::EzspFrame(Bytes::from_static(&[0x01, 0x02, 0x03]))
        )
    }
}
