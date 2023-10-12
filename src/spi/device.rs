use std::{io::ErrorKind, time::Duration};

use bytes::{Buf, BytesMut};
use nom::{Err, Needed};
use popol::{interest, Event, Sources};
use spidev::{Spidev, SpidevTransfer};
use sysfs_gpio::{AsyncPinPoller, Pin};

use super::{
    command::Command,
    error::{Error, Result},
    response::Response,
};

#[derive(Debug, Clone, Copy)]
pub enum State {
    Normal,
    Bootloader,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
enum Key {
    Interupt,
}

// TODO: Replace concrete pin and spidev impls with generics for testing
#[derive(Debug)]
pub struct Device {
    spi: Spidev,
    cs: Pin,
    int: AsyncPinPoller,
    reset: Pin,
    wake: Pin,
    state: State,
    sources: Sources<Key>,
    events: Vec<Event<Key>>,
    read_buf: BytesMut,
}

impl Device {
    pub fn new(spi: Spidev, cs: Pin, int: AsyncPinPoller, reset: Pin, wake: Pin) -> Device {
        let mut sources = Sources::new();
        sources.register(Key::Interupt, &int, interest::READ);
        Device {
            spi,
            cs,
            int,
            reset,
            wake,
            state: State::Unknown,
            sources,
            events: Vec::with_capacity(1),
            read_buf: BytesMut::zeroed(1024),
        }
    }

    fn read_bytes(&mut self, buf: &mut [u8]) -> Result<()> {
        let mut transfer = SpidevTransfer::read(buf);
        transfer.cs_change = 0;

        self.spi.transfer(&mut transfer)?;
        Ok(())
    }

    fn write_bytes(&mut self, buf: &[u8]) -> Result<()> {
        let mut transfer = SpidevTransfer::write(buf);
        transfer.cs_change = 0;

        self.spi.transfer(&mut transfer)?;
        Ok(())
    }

    fn read_response(&mut self) -> Result<Response> {
        // Read and discard 0xFF bytes until a different byte is encountered.
        let mut pos = 0;
        self.read_buf[0] = 0xFF;
        while self.read_buf[0] == 0xFF {
            let subslice = &mut self.read_buf.clone()[..0];
            self.read_bytes(subslice)?;
        }
        pos += 1;

        // Start parsing a response from the first byte
        loop {
            let input = self.read_buf.clone().freeze().into();
            match Response::parse(input) {
                Ok((_rest, res)) => {
                    self.cs.set_value(0)?;
                    self.read_buf.advance(pos);
                    return Ok(res);
                }
                Err(Err::Incomplete(Needed::Size(size))) => {
                    // The response is incomplete, allocate and read the bytes
                    // into the write section of the buffer.
                    let additional: usize = size.into();
                    self.read_buf.reserve(additional);
                    let end = pos + additional;
                    let subslice = &mut self.read_buf.clone()[pos..=end];
                    self.read_bytes(subslice)?;
                    pos = end;
                }
                Err(_) => {
                    self.cs.set_value(0)?;
                    self.read_buf.advance(pos);
                    return Err(Error::InvalidResponse);
                }
            }
        }
    }

    fn check_state(&self) -> Result<()> {
        match self.state {
            State::Unknown => Err(Error::NeedsReset),
            _ => Ok(()),
        }
    }

    // TODO: Replace concrete impl of poll with a generic for testing
    fn poll_interrupt(&mut self, timeout: Duration) -> Result<bool> {
        if let Err(e) = self.sources.poll(&mut self.events, timeout) {
            match e.kind() {
                ErrorKind::TimedOut => Ok(false),
                _ => Err(Error::from(e)),
            }
        } else if let Some(e) = self.events.drain(..).next() {
            Ok(e.is_readable())
        } else {
            Ok(false)
        }
    }

    /// Get the state of the device.
    ///
    /// This is not the true state of the device, but the last known state.
    pub fn state(&self) -> State {
        self.state
    }

    /// Returns true if the last known state is able accept commands.
    pub fn is_ready(&self) -> bool {
        !matches!(self.state, State::Unknown)
    }

    /// Poll for a callback. The call will timeout if a callback is not
    /// available from the device.
    pub fn poll_callback(&mut self, timeout: Duration) -> Result<Option<Response>> {
        self.check_state()?;

        if self.poll_interrupt(timeout)? {
            self.read_response().map(Some)
        } else {
            Ok(None)
        }
    }

    /// Write a command to the SPI bus and wait for a response.
    ///
    /// If the device is in bootloader mode and the command is an EZSP frame,
    /// an `Error:UnsupportedSpiCommand` will be returned.
    ///
    /// If the device is sleeping, an `Error::Unresponsive` will be returned.
    pub fn send(&mut self, command: &Command) -> Result<Response> {
        self.check_state()?;

        self.cs.set_value(1)?;

        let mut buf = BytesMut::zeroed(command.size());
        command.serialize(&mut buf);
        self.write_bytes(&buf.freeze())?;

        if self.poll_interrupt(Duration::from_millis(350))? {
            self.read_response()
        } else {
            self.state = State::Unknown;
            Err(Error::Unresponsive)
        }
    }

    /// Reset the NCP and wait for the NCP to signal readiness.
    ///
    /// If the NCP fails to respond to the reset, an `Error::Unresponsive` is
    /// returned.
    pub fn reset(&mut self) -> Result<()> {
        todo!()
    }

    /// Reset the NCP into bootloader mode and wait for the NCP to signal
    /// readiness.
    ///
    /// If the NCP fails to respond to the reset, an `Error::Unresponsive` is
    /// returned.
    pub fn reset_to_bootloader(&mut self) -> Result<()> {
        todo!()
    }

    /// Wakeup the NCP and wait for the NCP to signal readiness.
    ///
    /// If the NCP fails to respond to the wakeup, an `Error::Unresponsive` is
    /// returned.
    pub fn wakeup(&mut self) -> Result<()> {
        todo!()
    }
}
