use std::{
    io::{self, ErrorKind},
    path::Path,
    time::Duration,
};

use gpiod::{
    Active, AsValues, AsValuesMut, Bias, Chip, EdgeDetect, Input, LineId, Lines, Masked, Options,
    Output,
};
use popol::{interest, Sources};
use spidev::{SpiModeFlags, Spidev, SpidevOptions, SpidevTransfer};

use super::traits::SpiDevice;
use crate::spi::error::Result;

const GPIO_CONSUMER_PREFIX: &'static str = "ezsp-spi-bridge";

fn setup_interrupt_pin(chip: &Chip, int_id: LineId) -> io::Result<Lines<Input>> {
    chip.request_lines(
        Options::input([int_id])
            .edge(EdgeDetect::Falling)
            .consumer(GPIO_CONSUMER_PREFIX),
    )
}

fn setup_output_pins(
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
}

fn configure_spi_dev(spi: &mut Spidev) -> io::Result<()> {
    let mut options = SpidevOptions::new();
    options.mode(SpiModeFlags::SPI_NO_CS);
    options.bits_per_word(8);
    options.max_speed_hz(2000);
    spi.configure(&options)
}

pub struct Peripheral {
    io: Spidev,
    interrupt: Lines<Input>,
    output_pins: Lines<Output>,
    poll: Sources<()>,
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
        let chip = Chip::new(path)?;
        let interrupt = setup_interrupt_pin(&chip, int_id)?;
        let output_pins = setup_output_pins(&chip, cs_id, reset_id, wake_id)?;
        let mut poll = Sources::new();
        poll.register((), &interrupt, interest::READ);

        Ok(Peripheral {
            io: spi,
            interrupt,
            output_pins,
            poll,
        })
    }

    fn spi_read(&self, buf: &mut [u8]) -> io::Result<()> {
        let mut transfer = SpidevTransfer::read(buf);
        transfer.cs_change = 0;
        self.io.transfer(&mut transfer)
    }

    fn spi_write(&self, buf: &[u8]) -> io::Result<()> {
        let mut transfer = SpidevTransfer::write(buf);
        transfer.cs_change = 0;
        self.io.transfer(&mut transfer)
    }
}

impl SpiDevice for Peripheral {
    fn read(&mut self, mut buf: &mut [u8]) -> io::Result<()> {
        let mut transfer = SpidevTransfer::read(&mut buf);
        transfer.cs_change = 0;
        self.io.transfer(&mut transfer)
    }

    fn write(&mut self, buf: &[u8]) -> io::Result<()> {
        let mut transfer = SpidevTransfer::write(&buf);
        transfer.cs_change = 0;
        self.io.transfer(&mut transfer)
    }

    fn set_cs_signal(&mut self, value: bool) -> io::Result<()> {
        let mut values: Masked<u8> = Default::default();
        values.set(0, Some(value));
        self.output_pins.set_values(values)
    }

    fn set_wake_signal(&mut self, value: bool) -> io::Result<()> {
        let mut values: Masked<u8> = Default::default();
        values.set(2, Some(value));
        self.output_pins.set_values(values)
    }

    fn set_reset_signal(&mut self, value: bool) -> io::Result<()> {
        let mut values: Masked<u8> = Default::default();
        values.set(1, Some(value));
        self.output_pins.set_values(values)
    }

    fn poll_interrupt_signal(&mut self, dur: Duration) -> io::Result<bool> {
        let mut events = Vec::new();

        match self.poll.wait_timeout(&mut events, dur) {
            Ok(_) => {
                self.interrupt.read_event()?;
                Ok(true)
            }
            Err(e) if e.kind() == ErrorKind::TimedOut => Ok(false),
            Err(e) => Err(e),
        }
    }

    fn get_interrupt_value(&mut self) -> io::Result<bool> {
        let values = [false; 1];
        let res = self.interrupt.get_values(values)?;
        Ok(res.get(0).unwrap_or(false))
    }
}
