use crate::{
    ash::{create_ash_stream, AshStream},
    spi::{SpiDevice, SpiDeviceHandle},
};
use anyhow::Result;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_stream::StreamExt;
use tracing::{debug, warn};

enum State {
    Connected,
    Error(u8),
}

pub async fn handle<T>(client: T, device: SpiDeviceHandle) -> Result<()>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    let mut uart = create_ash_stream(client);
    
    Ok(())
}
