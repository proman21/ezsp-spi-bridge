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
pub enum RawResponse {
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

impl RawResponse {
    pub fn parse(input: Buffer) -> ParserResult<RawResponse> {
        terminated(
            nom::branch::alt((
                RawResponse::parse_ncp_reset,
                RawResponse::parse_oversized_payload_frame,
                RawResponse::parse_aborted_transaction,
                RawResponse::parse_missing_frame_terminator,
                RawResponse::parse_unsupported_spi_command,
                RawResponse::parse_spi_protocol_version,
                RawResponse::parse_spi_status,
                RawResponse::parse_bootloader_frame,
                RawResponse::parse_ezsp_frame,
            )),
            tag([0xA7]),
        )(input)
    }

    fn parse_ncp_reset(input: Buffer) -> ParserResult<RawResponse> {
        preceded(
            tag([0x00]),
            map(take(1usize), |mut i: Buffer| RawResponse::NcpReset(i.get_u8())),
        )(input)
    }

    fn parse_oversized_payload_frame(input: Buffer) -> ParserResult<RawResponse> {
        preceded(
            tag([0x01]),
            value(RawResponse::OversizedPayloadFrame, take(1usize)),
        )(input)
    }

    fn parse_aborted_transaction(input: Buffer) -> ParserResult<RawResponse> {
        preceded(
            tag([0x02]),
            value(RawResponse::AbortedTransaction, take(1usize)),
        )(input)
    }

    fn parse_missing_frame_terminator(input: Buffer) -> ParserResult<RawResponse> {
        preceded(
            tag([0x03]),
            value(RawResponse::MissingFrameTerminator, take(1usize)),
        )(input)
    }

    fn parse_unsupported_spi_command(input: Buffer) -> ParserResult<RawResponse> {
        preceded(
            tag([0x04]),
            value(RawResponse::UnsupportedSpiCommand, take(1usize)),
        )(input)
    }

    fn parse_spi_protocol_version(input: Buffer) -> ParserResult<RawResponse> {
        bits::<_, _, Error<(Buffer, usize)>, _, _>(preceded(
            bits_tag(0b10, 2usize),
            map(bits_take(6usize), RawResponse::SpiProtocolVersion),
        ))(input)
    }

    fn parse_spi_status(input: Buffer) -> ParserResult<RawResponse> {
        bits::<_, _, Error<(Buffer, usize)>, _, _>(preceded(
            bits_tag(0x60, 7usize),
            map(bool, RawResponse::SpiStatus),
        ))(input)
    }

    fn parse_bootloader_frame(input: Buffer) -> ParserResult<RawResponse> {
        preceded(
            tag([0xFD]),
            map(flat_map(u8, take), |b: Buffer| {
                RawResponse::BootloaderFrame(b.into_inner())
            }),
        )(input)
    }

    fn parse_ezsp_frame(input: Buffer) -> ParserResult<RawResponse> {
        preceded(
            tag([0xFE]),
            map(flat_map(u8, take), |b: Buffer| {
                RawResponse::EzspFrame(b.into_inner())
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
        let (_rest, res) = RawResponse::parse(buf).unwrap();

        assert_eq!(res, RawResponse::NcpReset(0x02));
    }

    #[test]
    fn it_parse_oversized_payload_response() {
        let buf = Buffer::from_static(&[0x01, 0x00, 0xA7]);
        let (_rest, res) = RawResponse::parse(buf).unwrap();

        assert_eq!(res, RawResponse::OversizedPayloadFrame);
    }

    #[test]
    fn it_parses_aborted_transaction_response() {
        let buf = Buffer::from_static(&[0x02, 0x00, 0xA7]);
        let (_rest, res) = RawResponse::parse(buf).unwrap();

        assert_eq!(res, RawResponse::AbortedTransaction);
    }

    #[test]
    fn it_parses_missing_frame_terminator_response() {
        let buf = Buffer::from_static(&[0x03, 0x00, 0xA7]);
        let (_rest, res) = RawResponse::parse(buf).unwrap();

        assert_eq!(res, RawResponse::MissingFrameTerminator);
    }

    #[test]
    fn it_parses_unsupported_spi_command_response() {
        let buf = Buffer::from_static(&[0x04, 0x00, 0xA7]);
        let (_rest, res) = RawResponse::parse(buf).unwrap();

        assert_eq!(res, RawResponse::UnsupportedSpiCommand);
    }

    #[test]
    fn it_parses_spi_protocol_version_response() {
        let buf = Buffer::from_static(&[0xAA, 0xA7]);
        let (_rest, res) = RawResponse::parse(buf).unwrap();

        assert_eq!(res, RawResponse::SpiProtocolVersion(0x2A))
    }

    #[test]
    fn it_parses_spi_status_response() {
        let buf = Buffer::from_static(&[0xC1, 0xA7]);
        let (_rest, res) = RawResponse::parse(buf).unwrap();

        assert_eq!(res, RawResponse::SpiStatus(true));
    }

    #[test]
    fn it_parses_bootloader_frame_response() {
        let buf = Buffer::from_static(&[0xFD, 0x03, 0x01, 0x02, 0x03, 0xA7]);
        let (_rest, res) = RawResponse::parse(buf).unwrap();

        assert_eq!(
            res,
            RawResponse::BootloaderFrame(Bytes::from_static(&[0x01, 0x02, 0x03]))
        )
    }

    #[test]
    fn it_parses_ezsp_frame_response() {
        let buf = Buffer::from_static(&[0xFE, 0x03, 0x01, 0x02, 0x03, 0xA7]);
        let (_rest, res) = RawResponse::parse(buf).unwrap();

        assert_eq!(
            res,
            RawResponse::EzspFrame(Bytes::from_static(&[0x01, 0x02, 0x03]))
        )
    }
}
