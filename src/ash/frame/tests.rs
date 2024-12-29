use crate::ash::{frame::Frame, FrameNumber};
use bytes::BytesMut;
use nom::{Err, Needed};

#[test]
fn it_rejects_an_unknown_frame_type() {
    let buf = [0xFF];
    let res = Frame::parse(&buf).unwrap_err();

    assert!(matches!(res, Err::Error(_)));
}

#[test]
fn it_rejects_a_early_terminated_frame() {
    let buf = [0xC2, 0x02, 0x51, 0x7E];
    let res = Frame::parse(&buf);

    assert!(res.is_err());
}

#[test]
fn it_parses_a_valid_data_frame() {
    let buf = [0x25, 0x00, 0x00, 0x00, 0x02, 0x1A, 0xAD, 0x7E];
    let (rest, frame) = Frame::parse(&buf).unwrap();

    assert_eq!(rest.len(), 0);
    assert!(
        matches!(frame, Frame::Data { frm_num, re_tx, ack_num, body } if *frm_num == 2 && !re_tx && *ack_num == 5 && body.as_ref() == [0x00, 0x00, 0x00, 0x02])
    );
}

#[test]
fn it_parses_valid_ack_frames() {
    let buf = [0x81, 0x60, 0x59, 0x7E];
    let (rest, frame) = Frame::parse(&buf).unwrap();

    assert_eq!(rest.len(), 0);
    assert!(matches!(frame, Frame::Ack { res, n_rdy, ack_num } if !res && !n_rdy && *ack_num == 1));

    let buf = [0x8E, 0x91, 0xB6, 0x7E];
    let (_rest, frame) = Frame::parse(&buf).unwrap();
    assert_eq!(rest.len(), 0);
    assert!(matches!(frame, Frame::Ack { res, n_rdy, ack_num } if !res && n_rdy && *ack_num == 6));
}

#[test]
fn it_parses_a_valid_nak_frame() {
    let buf = [0xA6, 0x34, 0xDC, 0x7E];
    let (rest, frame) = Frame::parse(&buf).unwrap();

    assert_eq!(rest.len(), 0);
    assert!(matches!(frame, Frame::Nak { res, n_rdy, ack_num } if !res && !n_rdy && *ack_num == 6));

    let buf = [0xAD, 0x85, 0xB7, 0x7E];
    let (rest, frame) = Frame::parse(&buf).unwrap();

    assert_eq!(rest.len(), 0);
    assert!(matches!(frame, Frame::Nak { res, n_rdy, ack_num } if !res && n_rdy && *ack_num == 5));
}

#[test]
fn it_parses_a_valid_rst_frame() {
    let buf = [0xC0, 0x38, 0xBC, 0x7E];
    let (rest, frame) = Frame::parse(&buf).unwrap();

    assert_eq!(rest.len(), 0);
    assert!(matches!(frame, Frame::Rst));
}

#[test]
fn it_parses_a_valid_rst_ack_frame() {
    let buf = [0xC1, 0x02, 0x02, 0x9B, 0x7B, 0x7E];
    let (rest, frame) = Frame::parse(&buf).unwrap();

    assert_eq!(rest.len(), 0);
    assert!(matches!(frame, Frame::RstAck { version, code } if version == 0x02 && code == 0x02));
}

#[test]
fn it_parses_a_valid_error_frame() {
    let buf = [0xC2, 0x02, 0x51, 0xA8, 0xBD, 0x7E];
    let (rest, frame) = Frame::parse(&buf).unwrap();

    assert_eq!(rest.len(), 0);
    assert!(matches!(frame, Frame::Error { version, code } if version == 0x02 && code == 0x52));
}

#[test]
fn it_serializes_control_bytes_correctly() {
    let data_frame = Frame::data(
        FrameNumber::new_truncate(2),
        false,
        FrameNumber::new_truncate(5),
        BytesMut::new(),
    );
    assert_eq!(data_frame.flag(), 0x25);

    let ack_frame = Frame::ack(false, FrameNumber::new_truncate(6));
    assert_eq!(ack_frame.flag(), 0x8E);

    let nak_frame = Frame::nak(true, FrameNumber::new_truncate(5));
    assert_eq!(nak_frame.flag(), 0xAD);

    let rst_frame = Frame::Rst;
    assert_eq!(rst_frame.flag(), 0xC0);

    let rst_ack_frame = Frame::rst_ack(0x02, 0x02);
    assert_eq!(rst_ack_frame.flag(), 0xC1);

    let error_frame = Frame::error(0x02, 0x52);
    assert_eq!(error_frame.flag(), 0xC2);
}

#[test]
fn it_returns_correct_data_field_lens() {
    let data_frame = Frame::data(
        FrameNumber::new_truncate(2),
        false,
        FrameNumber::new_truncate(5),
        BytesMut::new(),
    );
    assert!(matches!(data_frame.data_len(), Needed::Unknown));

    let ack_frame = Frame::ack(true, FrameNumber::new_truncate(6));
    assert!(matches!(ack_frame.data_len(), Needed::Size(size) if size.get() == 2));

    let nak_frame = Frame::nak(true, FrameNumber::new_truncate(6));
    assert!(matches!(nak_frame.data_len(), Needed::Size(size) if size.get() == 2));

    let rst_frame = Frame::Rst;
    assert!(matches!(rst_frame.data_len(), Needed::Size(size) if size.get() == 2));

    let rst_ack_frame = Frame::rst_ack(0x02, 0x02);
    assert!(matches!(rst_ack_frame.data_len(), Needed::Size(size) if size.get() == 4));

    let error_frame = Frame::error(0x02, 0x52);
    assert!(matches!(error_frame.data_len(), Needed::Size(size) if size.get() == 4));
}

#[test]
fn it_serializes_the_data_field_correctly() {
    let data_frame = Frame::data(
        FrameNumber::new_truncate(2),
        false,
        FrameNumber::new_truncate(5),
        BytesMut::from(&[0x00, 0x00, 0x00, 0x02][..]),
    );
    let mut buf = BytesMut::new();
    data_frame.serialize_data(&mut buf);
    assert_eq!(*buf, [0x42, 0x21, 0xA8, 0x56]);

    let ack_frame = Frame::ack(true, FrameNumber::new_truncate(6));
    buf = BytesMut::new();
    ack_frame.serialize_data(&mut buf);
    assert_eq!(buf.len(), 0);

    let nak_frame = Frame::nak(true, FrameNumber::new_truncate(6));
    buf = BytesMut::new();
    nak_frame.serialize_data(&mut buf);
    assert_eq!(buf.len(), 0);

    let rst_frame = Frame::Rst;
    buf = BytesMut::new();
    rst_frame.serialize_data(&mut buf);
    assert_eq!(buf.len(), 0);

    let rst_ack_frame = Frame::rst_ack(0x02, 0x02);
    buf = BytesMut::with_capacity(2);
    rst_ack_frame.serialize_data(&mut buf);
    assert_eq!(*buf, [0x02, 0x02]);

    let error_frame = Frame::error(0x02, 0x52);
    buf = BytesMut::with_capacity(2);
    error_frame.serialize_data(&mut buf);
    assert_eq!(*buf, [0x02, 0x52]);
}
