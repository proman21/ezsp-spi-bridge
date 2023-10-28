use std::time::Duration;

use bytes::{Buf, Bytes, BytesMut};
use nom::{Err, Finish, Needed};
use tokio::time::{sleep_until, Instant};

use crate::buffers::Buffer;

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

    async fn read_response(&mut self) -> Result<RawResponse> {
        // Read and discard 0xFF bytes until a different byte is encountered.
        let mut index = 0;
        self.read_buf[0] = self.device.drop_until().await?;
        index += 1;

        // Start parsing a response from the first byte
        let res: Result<(Buffer, RawResponse)> = loop {
            let input = self.read_buf.clone().freeze().into();
            let parse_res = RawResponse::parse(input);

            if let Err(Err::Incomplete(needed)) = parse_res {
                if let Needed::Size(size) = needed {
                    // The response is incomplete, allocate and read the bytes
                    // into the write section of the buffer.
                    let additional: usize = size.into();
                    self.read_buf.reserve(additional);
                    let mut subslice = self.read_buf.clone();
                    subslice.advance(index);
                    subslice.truncate(additional);
                    self.device.read(subslice).await?;
                    index += additional;
                } else {
                    break Err(Error::InvalidResponse);
                }
            }

            break parse_res.finish().map_err(|_| Error::InvalidResponse);
        };

        self.device.set_cs_signal(false).await?;
        self.read_buf.advance(index);

        res.map(|(_rest, res)| res)
    }

    fn check_state(&self) -> Result<()> {
        match self.state {
            State::Unknown => Err(Error::NeedsReset),
            _ => Ok(()),
        }
    }

    pub async fn poll_interrupt(&mut self, timeout: Duration) -> Result<bool> {
        let res = self.device.poll_interrupt_signal(timeout).await?;
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
    pub async fn send(&mut self, data: Bytes) -> Result<Bytes> {
        let command = if self.is_bootloader() {
            Command::BootloaderFrame(data)
        } else {
            Command::EzspFrame(data)
        };

        match self.send_command(&command).await? {
            SuccessResponse::BootloaderFrame(inner) | SuccessResponse::EzspFrame(inner) => {
                Ok(inner)
            }
            _ => unreachable!(),
        }
    }

    async fn send_command(&mut self, command: &Command) -> Result<SuccessResponse> {
        self.check_state()?;
        sleep_until(self.last_command_time + INTER_COMMAND_SPACING).await;

        self.device.set_cs_signal(true).await?;

        let mut buf = BytesMut::with_capacity(command.size());
        command.serialize(&mut buf);
        self.device.write(buf.freeze()).await?;

        if !self.device.poll_interrupt_signal(RESPONSE_TIMEOUT).await? {
            self.state = State::Unknown;
            return Err(Error::Unresponsive);
        }

        let res = self.read_response().await?;
        self.last_command_time = Instant::now();

        res.into()
    }

    async fn pulse_reset(&mut self, wake: bool) -> Result<()> {
        let start_time = Instant::now();
        self.device.set_reset_signal(true).await?;
        self.device.set_wake_signal(wake).await?;
        while start_time.elapsed().as_micros() < 2 {}
        self.device.set_reset_signal(false).await?;
        Ok(())
    }

    /// Reset the NCP, optionally into bootloader mode, and wait for the NCP to signal readiness.
    ///
    /// If the NCP fails to respond to the reset, an `Error::Unresponsive` is
    /// returned.
    pub async fn reset(&mut self, bootloader: bool) -> Result<()> {
        self.pulse_reset(bootloader).await?;
        self.state = State::Unknown;

        if !self
            .device
            .poll_interrupt_signal(RESET_STARTUP_TIME)
            .await?
        {
            return Err(Error::Unresponsive);
        }
        self.device.set_wake_signal(false).await?;

        let version_command = Command::SpiProtocolVersion;
        if !matches!(
            self.send_command(&version_command).await,
            Err(Error::UnexpectedReset(0x02))
        ) {
            return Err(Error::InvalidResponse);
        }

        if !matches!(
            self.send_command(&version_command).await?,
            SuccessResponse::SpiProtocolVersion(2)
        ) {
            return Err(Error::InvalidResponse);
        }

        if !matches!(
            self.send_command(&Command::SpiStatus).await?,
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
    pub async fn wakeup(&mut self) -> Result<()> {
        self.device.set_wake_signal(true).await?;

        if !self
            .device
            .poll_interrupt_signal(WAKE_HANDSHAKE_TIMEOUT)
            .await?
        {
            self.state = State::Unknown;
            return Err(Error::Unresponsive);
        }

        self.device.set_wake_signal(false).await?;
        Ok(())
    }
}
