use std::ops::Deref;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameNumber(u8);

impl FrameNumber {
    pub fn new_truncate(value: u8) -> FrameNumber {
        FrameNumber(value & 0x07)
    }
}

impl Deref for FrameNumber {
    type Target = u8;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TryFrom<u8> for FrameNumber {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value > 7 {
            Err(())
        } else {
            Ok(FrameNumber(value))
        }
    }
}

impl From<FrameNumber> for u8 {
    fn from(val: FrameNumber) -> Self {
        val.0
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
