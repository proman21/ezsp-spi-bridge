use tokio::io::{AsyncRead, AsyncWrite};

use crate::ash::{create_ash_stream, AshStream};

pub struct Bridge<T: AsyncRead + AsyncWrite> {
    uart: AshStream<T>,
}

impl<T: AsyncRead + AsyncWrite> Bridge<T> {
    pub fn new(client: T) -> Bridge<T> {
        Bridge {
            uart: create_ash_stream(client),
        }
    }

    pub async fn handle(&mut self) {
        loop {}
    }
}
