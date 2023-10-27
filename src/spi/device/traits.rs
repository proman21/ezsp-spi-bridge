use std::io::Result;
use std::time::Duration;

use async_trait::async_trait;
use bytes::{BytesMut, Bytes};
use mockall::automock;

#[automock]
#[async_trait]
pub trait SpiDevice {
    async fn drop_until(&mut self) -> Result<u8>;
    async fn read(&mut self, buf: BytesMut) -> Result<()>;
    async fn write(&mut self, buf: Bytes) -> Result<()>;
    async fn set_cs_signal(&mut self, value: bool) -> Result<()>;
    async fn set_wake_signal(&mut self, value: bool) -> Result<()>;
    async fn set_reset_signal(&mut self, value: bool) -> Result<()>;
    async fn poll_interrupt_signal(&mut self, dur: Duration) -> Result<bool>;
}
