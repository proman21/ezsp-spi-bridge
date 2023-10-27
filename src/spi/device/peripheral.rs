use std::{io, path::Path, time::Duration};

use async_trait::async_trait;
use bytes::{Bytes, BytesMut};
use spidev::{SpiModeFlags, Spidev, SpidevOptions};
use tokio::time::timeout;
use tokio_gpiod::{
    Active, AsValuesMut, Bias, Chip, EdgeDetect, Input, LineId, Lines, Options, Output,
};

use super::{traits::SpiDevice, DeviceIoHandle};
use crate::spi::error::Result;

const GPIO_CONSUMER_PREFIX: &'static str = "ezsp-spi-bridge";

async fn setup_interrupt_pin(chip: &Chip, int_id: LineId) -> io::Result<Lines<Input>> {
    chip.request_lines(
        Options::input([int_id])
            .edge(EdgeDetect::Falling)
            .consumer(GPIO_CONSUMER_PREFIX),
    )
    .await
}

async fn setup_output_pins(
    chip: &Chip,
    cs_id: LineId,
    reset_id: LineId,
    wake_id: LineId,
) -> io::Result<Lines<Output>> {
    chip.request_lines(
        Options::output([cs_id, reset_id, wake_id])
            .bias(Bias::PullUp)
            .active(Active::Low)
            .consumer(GPIO_CONSUMER_PREFIX),
    )
    .await
}

fn configure_spi_dev(spi: &mut Spidev) -> io::Result<()> {
    let mut options = SpidevOptions::new();
    options.mode(SpiModeFlags::SPI_NO_CS);
    options.bits_per_word(8);
    options.max_speed_hz(2000);
    spi.configure(&options)
}

pub struct Peripheral {
    io: DeviceIoHandle,
    interrupt: Lines<Input>,
    output_pins: Lines<Output>,
}

impl Peripheral {
    pub async fn new(
        mut spi: Spidev,
        path: impl AsRef<Path>,
        cs_id: LineId,
        int_id: LineId,
        reset_id: LineId,
        wake_id: LineId,
    ) -> Result<Peripheral> {
        configure_spi_dev(&mut spi)?;
        let io = DeviceIoHandle::new(spi);
        let chip = Chip::new(path).await?;
        let interrupt = setup_interrupt_pin(&chip, int_id).await?;
        let output_pins = setup_output_pins(&chip, cs_id, reset_id, wake_id).await?;
        Ok(Peripheral {
            io,
            interrupt,
            output_pins,
        })
    }
}

#[async_trait]
impl SpiDevice for Peripheral {
    async fn drop_until(&mut self) -> io::Result<u8> {
        self.io.drop_until_byte().await
    }

    async fn read(&mut self, buf: BytesMut) -> io::Result<()> {
        self.io.read_bytes(buf).await
    }

    async fn write(&mut self, buf: Bytes) -> io::Result<()> {
        self.io.write_bytes(buf).await
    }

    async fn set_cs_signal(&mut self, value: bool) -> io::Result<()> {
        let mut values = [None; 3];
        values.set(0, Some(value));
        self.output_pins.set_values(values).await
    }

    async fn set_wake_signal(&mut self, value: bool) -> io::Result<()> {
        let mut values = [None; 3];
        values.set(2, Some(value));
        self.output_pins.set_values(values).await
    }

    async fn set_reset_signal(&mut self, value: bool) -> io::Result<()> {
        let mut values = [None; 3];
        values.set(1, Some(value));
        self.output_pins.set_values(values).await
    }

    async fn poll_interrupt_signal(&mut self, dur: Duration) -> io::Result<bool> {
        timeout(dur, self.interrupt.read_event())
            .await
            .map_or(Ok(false), |_| Ok(true))
    }
}
