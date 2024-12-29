mod checksum;
mod codec;
mod constants;
mod error;
mod frame;
mod protocol;
mod types;

pub use error::{Error, Result};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::Framed;
pub use types::FrameNumber;

use self::codec::AshCodec;

pub type AshStream<T> = Framed<T, AshCodec>;

pub fn create_ash_stream<T: AsyncRead + AsyncWrite>(inner: T) -> AshStream<T> {
    Framed::with_capacity(inner, AshCodec::default(), 2048)
}