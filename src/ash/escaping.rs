use bytes::BufMut;

use super::frame::{ESCAPE_BYTE, FLAG_BYTE, SUB_BYTE, CANCEL_BYTE};

const RESERVED_BYTES: [u8; 6] = [FLAG_BYTE, ESCAPE_BYTE, 0x11, 0x13, SUB_BYTE, CANCEL_BYTE];

pub fn escape_reserved_bytes(frame: &[u8], mut buf: &mut [u8]) {
    for item in frame.split_inclusive(|x| RESERVED_BYTES.contains(x)) {
        if let Some((byte, rest)) = item.split_last() {
            buf.put_slice(rest);
            buf.put_u8(ESCAPE_BYTE);
            buf.put_u8(byte ^ 0x20);
        }
    }
}

pub fn unescape_reserved_bytes(buf: &mut [u8]) {
    let mut escape_next = false;
    for byte in buf {
        if escape_next {
            if !RESERVED_BYTES.contains(byte) {
                *byte ^= 0x20;
            }
            escape_next = false;
        } else {
            escape_next = *byte == ESCAPE_BYTE
        }
    }
}

#[cfg(test)]
mod tests {
    use bytes::BytesMut;

    use super::*;

    #[test]
    fn it_escapes_reserved_bytes() {
        let frame = [0x00, 0x7E, 0x7D, 0x11, 0x13, 0x18, 0x1A];
        let mut buf = BytesMut::with_capacity(frame.len() * 2);
        let res = [
            0x00, 0x7D, 0x5E, 0x7D, 0x5D, 0x7D, 0x31, 0x7D, 0x33, 0x7D, 0x38, 0x7D, 0x3A,
        ];

        escape_reserved_bytes(&frame, &mut buf);
        assert_eq!(buf.as_ref(), res);
    }

    #[test]
    fn it_unescapes_reserved_bytes() {
        let mut buf = [
            0x00, 0x7D, 0x5E, 0x7D, 0x5D, 0x7D, 0x31, 0x7D, 0x33, 0x7D, 0x38, 0x7D, 0x3A,
        ];

        unescape_reserved_bytes(&mut buf);
        assert_eq!(
            buf,
            [0x00, 0x7D, 0x7E, 0x7D, 0x7D, 0x7D, 0x11, 0x7D, 0x13, 0x7D, 0x18, 0x7D, 0x1A]
        );
    }
}
