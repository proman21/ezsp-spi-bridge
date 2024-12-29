pub const FLAG_BYTE: u8 = 0x7E;
pub const SUB_BYTE: u8 = 0x18;
pub const CANCEL_BYTE: u8 = 0x1A;
pub const ESCAPE_BYTE: u8 = 0x7D;

pub const RESERVED_BYTES: [u8; 6] = [FLAG_BYTE, ESCAPE_BYTE, 0x11, 0x13, SUB_BYTE, CANCEL_BYTE];

pub const RESET_UNKNOWN: u8 = 0x00;
pub const RESET_EXTERNAL: u8 = 0x01;
pub const RESET_POWERON: u8 = 0x02;
pub const RESET_WATCHDOG: u8 = 0x03;
pub const RESET_ASSERT: u8 = 0x06;
pub const RESET_BOOTLOADER: u8 = 0x09;
pub const RESET_SOFTWARE: u8 = 0x0B;
pub const ERROR_MAX_ACK_TIMEOUT: u8 = 0x51;
pub const ERROR_CUSTOM: u8 = 0x80;

pub const ASH_VERSION_2: u8 = 0x02;
