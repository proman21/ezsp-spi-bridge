use spidev::Spidev;
use sysfs_gpio::Pin;

pub struct Inner {
    spi: Spidev,
    host_int: Pin,
    reset: Pin,
    wake: Pin,
}