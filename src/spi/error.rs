#[derive(Debug)]
pub enum Error {
    InvalidResponse,
    Io(std::io::Error),
    NeedsReset,
    Unresponsive,
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::Io(value)
    }
}

impl From<sysfs_gpio::Error> for Error {
    fn from(value: sysfs_gpio::Error) -> Self {
        match value {
            sysfs_gpio::Error::Io(e) => Error::Io(e),
            sysfs_gpio::Error::Unexpected(s) => {
                Error::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, s))
            }
            sysfs_gpio::Error::InvalidPath(s) => {
                Error::Io(std::io::Error::new(std::io::ErrorKind::NotFound, s))
            }
            sysfs_gpio::Error::Unsupported(s) => {
                Error::Io(std::io::Error::new(std::io::ErrorKind::Unsupported, s))
            }
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
