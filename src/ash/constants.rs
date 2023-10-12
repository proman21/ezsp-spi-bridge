pub const FLAG_BYTE: u8 = 0x7E;
pub const SUB_BYTE: u8 = 0x18;
pub const CANCEL_BYTE: u8 = 0x1A;
pub const ESCAPE_BYTE: u8 = 0x7D;

pub const RESERVED_BYTES: [u8; 6] = [FLAG_BYTE, ESCAPE_BYTE, 0x11, 0x13, SUB_BYTE, CANCEL_BYTE];
