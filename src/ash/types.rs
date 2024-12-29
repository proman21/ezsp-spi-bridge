use std::{
    fmt::Display,
    ops::{Add, AddAssign, Deref},
};

fn three_bit_wrapped_add(lhs: u8, rhs: u8) -> u8 {
    (lhs + rhs) % 8
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameNumber(u8);

impl FrameNumber {
    pub fn new(value: u8) -> Option<FrameNumber> {
        if value > 7 {
            None
        } else {
            Some(FrameNumber(value))
        }
    }

    pub fn new_truncate(value: u8) -> FrameNumber {
        FrameNumber(value & 0x07)
    }

    pub fn zero() -> FrameNumber {
        FrameNumber(0)
    }
}

impl Deref for FrameNumber {
    type Target = u8;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TryFrom<u8> for FrameNumber {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        FrameNumber::new(value).ok_or("FrameNumber only accepts values between 0 and 7")
    }
}

impl From<FrameNumber> for u8 {
    fn from(val: FrameNumber) -> Self {
        val.0
    }
}

impl Default for FrameNumber {
    fn default() -> Self {
        Self(0)
    }
}

impl Add<u8> for FrameNumber {
    type Output = FrameNumber;

    fn add(self, rhs: u8) -> Self::Output {
        FrameNumber(three_bit_wrapped_add(self.0, rhs))
    }
}

impl AddAssign<u8> for FrameNumber {
    fn add_assign(&mut self, rhs: u8) {
        self.0 = three_bit_wrapped_add(self.0, rhs);
    }
}

impl Display for FrameNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

#[cfg(test)]
mod tests {
    use super::FrameNumber;

    #[test]
    fn it_accepts_a_valid_frame_number() {
        let res = FrameNumber::try_from(7);
        assert!(res.is_ok());
    }

    #[test]
    fn it_rejects_invalid_frame_number() {
        let res = FrameNumber::try_from(42);
        assert!(res.is_err())
    }

    #[test]
    fn it_truncates_invalid_frame_number() {
        let res = FrameNumber::new_truncate(0xBE);
        assert_eq!(*res, 6);
    }
}
