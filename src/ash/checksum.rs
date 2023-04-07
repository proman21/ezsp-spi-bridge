use crc::{Crc, CRC_16_XMODEM};

const CRC_CCITT: Crc<u16> = Crc::<u16>::new(&CRC_16_XMODEM);

pub fn frame_checksum(frame: &[u8]) -> u16 {
    let mut digester = CRC_CCITT.digest_with_initial(0xFFFF);
    digester.update(frame);
    digester.finalize()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_computes_checksum_for_rst_frame() {
        let rst_frame = [0xC0];
        assert_eq!(frame_checksum(&rst_frame), 0x38BC);
    }

    #[test]
    fn it_computes_checksum_for_rstack_frame() {
        let rstack_frame = [0xC1, 0x02, 0x02];
        assert_eq!(frame_checksum(&rstack_frame), 0x9B7B);
    }

    #[test]
    fn it_computes_checksum_for_error_frame() {
        let error_frame = [0xC2, 0x01, 0x52];
        assert_eq!(frame_checksum(&error_frame), 0xCD8D);
    }

    #[test]
    fn it_computes_checksum_for_data_frames() {
        let data_frame_1 = [0x25, 0x00, 0x00, 0x00, 0x02];
        assert_eq!(frame_checksum(&data_frame_1), 0x1AAD);

        let data_frame_2 = [0x53, 0x00, 0x80, 0x00, 0x02, 0x02, 0x11, 0x30];
        assert_eq!(frame_checksum(&data_frame_2), 0x6316);

        let data_frame_3 = [0x25, 0x42, 0x21, 0xA8, 0x56];
        assert_eq!(frame_checksum(&data_frame_3), 0xA609);

        let data_frame_4 = [0x53, 0x42, 0xA1, 0xA8, 0x56, 0x28, 0x04, 0x82];
        assert_eq!(frame_checksum(&data_frame_4), 0x032A);
    }

    #[test]
    fn it_computes_checksum_for_ack_frames() {
        let ack_frame_1 = [0x81];
        assert_eq!(frame_checksum(&ack_frame_1), 0x6059);

        let ack_frame_2 = [0x8E];
        assert_eq!(frame_checksum(&ack_frame_2), 0x91B6);
    }

    #[test]
    fn it_computes_checksum_for_nack_frames() {
        let nack_frame_1 = [0xA6];
        assert_eq!(frame_checksum(&nack_frame_1), 0x34DC);

        let nack_frame_2 = [0xAD];
        assert_eq!(frame_checksum(&nack_frame_2), 0x85B7);
    }
}
