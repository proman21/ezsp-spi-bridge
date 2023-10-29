use std::io::Result;
use std::time::Duration;

use mockall::automock;

#[automock]
pub trait SpiDevice {
    fn read(&mut self, buf: &mut [u8]) -> Result<()>;
    fn write(&mut self, buf: &[u8]) -> Result<()>;
    fn set_cs_signal(&mut self, value: bool) -> Result<()>;
    fn set_wake_signal(&mut self, value: bool) -> Result<()>;
    fn set_reset_signal(&mut self, value: bool) -> Result<()>;
    fn poll_interrupt_signal(&mut self, dur: Duration) -> Result<bool>;
    fn get_interrupt_value(&mut self) -> Result<bool>;
}
