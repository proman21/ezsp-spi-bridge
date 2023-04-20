use super::{
    command::Command, error::Result, inner::Inner, reset::Reset, response::Response,
    transaction::Transaction, wakeup::Wakeup,
};

pub struct Ncp(Inner);

// Read and discard 0xFF bytes until a different byte is encountered.
// Read the SPI byte and determine the type of response
// If the next byte is supposed to be an error code or single byte response, read it
// If the next byte is supposed to be the length byte, read, validate, and then read out that many bytes
// Read and validate the next byte as the frame terminator

impl Ncp {
    /// Attempt to read a callback response from the NCP
    pub fn read_callback(&mut self) -> Result<Response> {
        todo!()
    }

    /// Write a command to the SPI bus and wait for a response
    ///
    /// The connection will be borrowed so that another transaction cannot
    /// be established while waiting for a response
    pub fn start_transaction(&mut self, command: &Command) -> Result<Transaction> {
        todo!()
    }

    /// Reset the NCP and wait for the NCP to signal readiness
    pub fn reset(&mut self) -> Result<Reset> {
        todo!()
    }

    /// Reset the NCP into bootloader mode and wait for the NCP to signal readiness
    pub fn reset_to_bootloader(&mut self) -> Result<Reset> {
        todo!()
    }

    /// Wakeup the NCP and wait for the NCP to signal readiness
    pub fn wakeup(&mut self) -> Result<Wakeup> {
        todo!()
    }
}
