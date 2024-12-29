use super::{
    constants::{CANCEL_BYTE, FLAG_BYTE, SUB_BYTE},
    frame::Frame,
    Error, Result,
};
use bytes::{Buf, BytesMut};
use nom::{Err, Finish, Needed, Offset};
use tokio_util::codec::{Decoder, Encoder};
use tracing::{instrument, trace};

#[derive(Debug)]
pub struct AshCodec {
    dropping: bool,
}

impl AshCodec {
    /// Locate unescaped cancel or substitute bytes and drop the portion of the
    /// buffer up to and including the detected bytes.
    ///
    /// If a substitute byte is encountered, subsequent calls to this function
    /// will continue dropping the buffer preceding an unescaped flag byte.
    #[instrument]
    fn drop_buffer_framing_errors(&mut self, buf: &mut BytesMut) {
        trace!("Searching and dropping buffer framing errors");
        if !self.dropping {
            self.drop_buffer_before_substitute(buf);
        }

        while self.dropping && buf.len() > 0 {
            self.drop_buffer_til_flag(buf);

            self.drop_buffer_before_substitute(buf);
        }
    }

    #[instrument]
    fn drop_buffer_before_substitute(&mut self, buf: &mut BytesMut) {
        trace!("Searching for framing error bytes");
        loop {
            if let Some(idx) = buf
                .iter()
                .position(|&b| b == SUB_BYTE || b == CANCEL_BYTE || b == FLAG_BYTE)
            {
                if buf[idx] == FLAG_BYTE {
                    trace!("Flag byte detected at index {}, bailing", idx);
                    break;
                }
                self.dropping = buf[idx] == SUB_BYTE;
                trace!(
                    dropping = self.dropping,
                    "Found a framing byte {:x} at index {}",
                    buf[idx],
                    idx
                );
                buf.advance(idx + 1);
            }
        }
    }

    fn drop_buffer_til_flag(&mut self, buf: &mut BytesMut) {
        trace!("Dropping buffer until flag byte found");
        if let Some(idx) = buf.iter().position(|&b| b == FLAG_BYTE) {
            trace!("Flag byte found at pos {}, dropping bytes before", idx);
            buf.advance(idx + 1);
            self.dropping = false;
            trace!("Buffer drop operation complete")
        } else {
            let _ = buf.split();
        }
    }

    pub fn is_dropping(&self) -> bool {
        self.dropping
    }
}

impl Default for AshCodec {
    fn default() -> Self {
        AshCodec { dropping: false }
    }
}

impl Decoder for AshCodec {
    type Item = Result<Frame>;
    type Error = Error;

    #[instrument]
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>> {
        self.drop_buffer_framing_errors(src);

        let res = Frame::parse(&src[..]);

        if let Err(Err::Incomplete(needed)) = res {
            trace!(bytes_needed = ?needed, "Incomplete frame detected");
            if let Needed::Size(additional) = needed {
                src.reserve(additional.into());
            }
            return Ok(None);
        }

        let (rest, frame) = match res.finish() {
            Ok(v) => v,
            Err(e) => {
                let (input, error) = e.into_inner();
                src.advance(src.offset(input));
                return Err(error);
            }
        };
        let offset = src.offset(rest);
        trace!("Frame decoded, {} bytes", offset);
        src.advance(offset);
        Ok(Some(Ok(frame)))
    }
}

impl Encoder<Frame> for AshCodec {
    type Error = Error;

    fn encode(&mut self, item: Frame, dst: &mut BytesMut) -> Result<()> {
        item.serialize(dst);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use bytes::BufMut;

    use super::*;

    #[test]
    fn it_decodes_a_valid_frame() {
        let mut buf: BytesMut = [0x25, 0x42, 0x21, 0xA8, 0x56, 0xA6, 0x09, 0x7E]
            .as_ref()
            .into();
        let mut codec = AshCodec::default();

        assert!(matches!(codec.decode(&mut buf), Ok(Some(Ok(_)))));
        assert_eq!(buf.len(), 0);
    }

    #[test]
    fn it_requests_more_data_when_incomplete_frame_detected() {
        let mut buf: BytesMut = [0x25, 0x42, 0x21, 0xA8].as_ref().into();
        let mut codec = AshCodec::default();

        assert!(matches!(codec.decode(&mut buf), Ok(None)));
        assert_eq!(buf.len(), 4);
        assert!(buf.capacity() > 5);
    }

    #[test]
    fn it_soft_fails_if_frame_checksum_is_invalid() {
        let mut buf: BytesMut = [0x25, 0x42, 0x21, 0xA8, 0x56, 0x00, 0x00, 0x7E]
            .as_ref()
            .into();
        let mut codec = AshCodec::default();

        assert!(matches!(
            codec.decode(&mut buf),
            Ok(Some(Err(Error::InvalidChecksum(_))))
        ));
        assert_eq!(buf.len(), 0);
    }

    #[test]
    fn it_soft_fails_if_frame_data_is_invalid() {
        let mut buf: BytesMut = [0xC2, 0x02, 0x51, 0x7E].as_ref().into();
        let mut codec = AshCodec::default();

        assert!(matches!(
            codec.decode(&mut buf),
            Ok(Some(Err(Error::InvalidDataField(_))))
        ));
        assert_eq!(buf.len(), 0);
    }

    #[test]
    fn it_hard_fails_if_invalid_control_byte_encountered() {
        let mut buf: BytesMut = [0xFF].as_ref().into();
        let mut codec = AshCodec::default();

        assert!(matches!(codec.decode(&mut buf), Err(Error::UnknownFrame)))
    }

    #[test]
    fn it_drops_buffer_before_cancel_byte() {
        let mut buf: BytesMut = [0xFF, 0xFF, 0xFF, 0x1A].as_ref().into();
        let mut codec = AshCodec::default();

        assert!(matches!(codec.decode(&mut buf), Ok(None)));
        assert!(!codec.is_dropping());
        assert_eq!(buf.len(), 0)
    }

    #[test]
    fn it_does_not_drop_frame_before_cancel_byte() {
        let mut buf: BytesMut = [
            0x25, 0x42, 0x21, 0xA8, 0x56, 0xA6, 0x09, 0x7E, 0xFF, 0xFF, 0xFF, 0x1A,
        ]
        .as_ref()
        .into();
        let mut codec = AshCodec::default();

        assert!(matches!(codec.decode(&mut buf), Ok(Some(Ok(_)))));
        assert_eq!(*buf, [0xFF, 0xFF, 0xFF, 0x1A]);
    }

    #[test]
    fn it_correctly_drops_buffer_before_and_after_substitute_byte() {
        let mut buf: BytesMut = [
            0xFF, 0xFF, 0xFF, 0x18, 0x25, 0x42, 0x21, 0xA8, 0x56, 0xA6, 0x09, 0x7E,
        ]
        .as_ref()
        .into();
        let mut codec = AshCodec::default();

        assert!(matches!(codec.decode(&mut buf), Ok(None)));
        assert_eq!(buf.len(), 0);
    }

    #[test]
    fn it_continues_dropping_buffer_after_substitute_bytes() {
        let mut buf: BytesMut = [0xFF, 0xFF, 0xFF, 0x18, 0x25, 0x42, 0x21].as_ref().into();
        let mut codec = AshCodec::default();

        assert!(matches!(codec.decode(&mut buf), Ok(None)));
        assert_eq!(buf.len(), 0);
        assert!(codec.is_dropping());

        buf.put_slice([0xA8, 0x56, 0xA6, 0x09, 0x7E].as_ref());
        assert!(matches!(codec.decode(&mut buf), Ok(None)));
        assert_eq!(buf.len(), 0);
        assert!(!codec.is_dropping());
    }
}
