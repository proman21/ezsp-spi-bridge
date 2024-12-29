mod command;
mod device;
mod error;
mod handle;
mod ncp;
mod response;

use anyhow::Result;
pub use device::Peripheral;
pub use device::SpiDevice;
pub use handle::{spi_device_handle, SpiDeviceActor, SpiDeviceHandle};
use spidev::Spidev;

use crate::settings::Spi;

pub async fn create_spi_peripheral(settings: &Spi) -> Result<Peripheral> {
    let spi = Spidev::open(&settings.device)?;
    Ok(Peripheral::new(
        spi,
        &settings.gpiochip,
        settings.cs_line,
        settings.int_line,
        settings.reset_line,
        settings.wake_line,
    )
    .await?)
}
