use std::time::{Duration, Instant};

use bytes::{Buf, Bytes, BytesMut};
use nom::{Err, Finish, Needed};

use super::{
    command::Command,
    device::SpiDevice,
    error::{Error, Result},
    response::RawResponse,
};

const RESPONSE_TIMEOUT: Duration = Duration::from_millis(350);
const RESET_PULSE_TIME: Duration = Duration::from_micros(26);
const RESET_STARTUP_TIME: Duration = Duration::from_millis(7500);
const INTER_COMMAND_SPACING: Duration = Duration::from_millis(1);
const WAKE_HANDSHAKE_TIMEOUT: Duration = Duration::from_millis(300);

#[derive(Debug, Clone, Copy)]
pub enum State {
    Normal,
    Bootloader,
    Unknown,
}

#[derive(Debug)]
pub enum SuccessResponse {
    EzspFrame(Bytes),
    BootloaderFrame(Bytes),
    SpiStatus(bool),
    SpiProtocolVersion(u8),
}

impl Into<Result<SuccessResponse>> for RawResponse {
    fn into(self) -> Result<SuccessResponse> {
        match self {
            RawResponse::AbortedTransaction
            | RawResponse::MissingFrameTerminator
            | RawResponse::UnsupportedSpiCommand => Err(Error::InternalError),
            RawResponse::OversizedPayloadFrame => Err(Error::OversizedPayload),
            RawResponse::NcpReset(code) => Err(Error::UnexpectedReset(code)),
            RawResponse::BootloaderFrame(inner) => Ok(SuccessResponse::BootloaderFrame(inner)),
            RawResponse::EzspFrame(inner) => Ok(SuccessResponse::EzspFrame(inner)),
            RawResponse::SpiProtocolVersion(inner) => {
                Ok(SuccessResponse::SpiProtocolVersion(inner))
            }
            RawResponse::SpiStatus(inner) => Ok(SuccessResponse::SpiStatus(inner)),
        }
    }
}

#[derive(Debug)]
pub struct NCP<D: SpiDevice> {
    device: D,
    state: State,
    read_buf: BytesMut,
    last_command_time: Instant,
}

impl<D: SpiDevice> NCP<D> {
    pub fn new(device: D) -> NCP<D> {
        NCP {
            device,
            state: State::Unknown,
            read_buf: BytesMut::with_capacity(1024),
            last_command_time: Instant::now(),
        }
    }

    fn read_response(&mut self) -> Result<RawResponse> {
        let mut write_buffer = self.read_buf.clone();
        // Read and discard 0xFF bytes until a different byte is encountered.
        write_buffer[0] = 0xFF;
        while self.read_buf[0] == 0xFF {
            self.device.read(&mut write_buffer[..1])?;
        }
        write_buffer.advance(1);

        // Start parsing a response from the first byte
        let res = self.try_parse_response(&mut write_buffer);
        self.device.set_cs_signal(false)?;
        self.read_buf = write_buffer;
        res
    }

    fn try_parse_response(&mut self, buffer: &mut BytesMut) -> Result<RawResponse> {
        loop {
            let input = self.read_buf.clone().freeze().into();
            let parse_res = RawResponse::parse(input);

            if let Err(Err::Incomplete(needed)) = parse_res {
                if let Needed::Size(size) = needed {
                    // The response is incomplete, allocate and read the bytes
                    // into the write buffer.
                    let additional: usize = size.into();
                    buffer.reserve(additional);
                    self.device.read(&mut buffer[..=additional])?;
                    buffer.advance(additional);
                } else {
                    return Err(Error::InvalidResponse);
                }
            } else {
                return parse_res
                    .finish()
                    .map_err(|_| Error::InvalidResponse)
                    .map(|(_, res)| res);
            }
        }
    }

    fn check_state(&self) -> Result<()> {
        match self.state {
            State::Unknown => Err(Error::NeedsReset),
            _ => Ok(()),
        }
    }

    pub fn has_callback(&mut self) -> Result<bool> {
        let res = self.device.get_interrupt_value()?;
        Ok(res)
    }

    /// Get the state of the device.
    ///
    /// This is not the true state of the device, but the last known state.
    pub fn state(&self) -> State {
        self.state
    }

    /// Returns true if the last known state is able to accept commands.
    pub fn is_ready(&self) -> bool {
        self.check_state().is_ok()
    }

    /// Returns true if the NCP is in bootloader mode.
    pub fn is_bootloader(&self) -> bool {
        matches!(self.state, State::Bootloader)
    }

    /// Write a frame to the SPI bus and wait for a response.
    ///
    /// If the device state is unknown, an 'Error::NeedsReset` will be returned.
    /// If the device is sleeping, an `Error::Unresponsive` will be returned.
    pub fn send(&mut self, data: Bytes) -> Result<Bytes> {
        let command = if self.is_bootloader() {
            Command::BootloaderFrame(data)
        } else {
            Command::EzspFrame(data)
        };

        match self.send_command(&command)? {
            SuccessResponse::BootloaderFrame(inner) | SuccessResponse::EzspFrame(inner) => {
                Ok(inner)
            }
            _ => unreachable!(),
        }
    }

    fn send_command(&mut self, command: &Command) -> Result<SuccessResponse> {
        self.check_state()?;
        while self.last_command_time.elapsed() < INTER_COMMAND_SPACING {}

        self.device.set_cs_signal(true)?;

        let mut buf = BytesMut::with_capacity(command.size());
        command.serialize(&mut buf);
        self.device.write(&buf.freeze())?;

        if !self.device.poll_interrupt_signal(RESPONSE_TIMEOUT)? {
            self.state = State::Unknown;
            return Err(Error::Unresponsive);
        }

        let res = self.read_response()?;
        self.last_command_time = Instant::now();

        res.into()
    }

    fn pulse_reset(&mut self, wake: bool) -> Result<()> {
        let start_time = Instant::now();
        self.device.set_reset_signal(true)?;
        self.device.set_wake_signal(wake)?;
        while start_time.elapsed() < RESET_PULSE_TIME {}
        self.device.set_reset_signal(false)?;
        Ok(())
    }

    /// Reset the NCP, optionally into bootloader mode, and wait for the NCP to signal readiness.
    ///
    /// If the NCP fails to respond to the reset, an `Error::Unresponsive` is
    /// returned.
    pub fn reset(&mut self, bootloader: bool) -> Result<()> {
        self.pulse_reset(bootloader)?;
        self.state = State::Unknown;

        if !self.device.poll_interrupt_signal(RESET_STARTUP_TIME)? {
            return Err(Error::Unresponsive);
        }
        self.device.set_wake_signal(false)?;

        let version_command = Command::SpiProtocolVersion;
        if !matches!(
            self.send_command(&version_command),
            Err(Error::UnexpectedReset(0x02))
        ) {
            return Err(Error::InvalidResponse);
        }

        if !matches!(
            self.send_command(&version_command)?,
            SuccessResponse::SpiProtocolVersion(2)
        ) {
            return Err(Error::InvalidResponse);
        }

        if !matches!(
            self.send_command(&Command::SpiStatus)?,
            SuccessResponse::SpiStatus(true)
        ) {
            return Err(Error::InvalidResponse);
        }

        self.state = if bootloader {
            State::Bootloader
        } else {
            State::Normal
        };

        Ok(())
    }

    /// Wakeup the NCP and wait for the NCP to signal readiness.
    ///
    /// If the NCP fails to respond to the wakeup, an `Error::Unresponsive` is
    /// returned.
    pub fn wakeup(&mut self) -> Result<()> {
        self.device.set_wake_signal(true)?;

        if !self.device.poll_interrupt_signal(WAKE_HANDSHAKE_TIMEOUT)? {
            self.state = State::Unknown;
            return Err(Error::Unresponsive);
        }

        self.device.set_wake_signal(false)?;
        Ok(())
    }

    pub fn into_inner(self) -> D {
        self.device
    }
}
