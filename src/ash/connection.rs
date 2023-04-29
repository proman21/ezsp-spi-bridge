use std::io::{Cursor, ErrorKind};

use bytes::{Buf, BytesMut};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufWriter};

use super::{
    error::{Error, Result},
    frame::{Frame, FrameFormat, CANCEL_BYTE, FLAG_BYTE, SUB_BYTE},
};
use crate::buffer::BufferMut;

/// A wrapper around a reader and writer that reads and writes ASH frames
pub struct Connection<S: AsyncRead + AsyncWrite + Unpin> {
    buffer: BufferMut,
    stream: BufWriter<S>,
    dropping: bool,
}

impl<S: AsyncRead + AsyncWrite + Unpin> Connection<S> {
    pub fn new(stream: S) -> Connection<S> {
        Self {
            buffer: BytesMut::with_capacity(2048).into(),
            stream: BufWriter::new(stream),
            dropping: false,
        }
    }

    pub fn into_inner(self) -> S {
        self.stream.into_inner()
    }

    /// Attempt to read a frame from the internal buffer, filling it with more
    /// data from the inner reader if none can be found
    pub async fn read_frame(&mut self) -> Result<Option<Frame>> {
        loop {
            if self.dropping {
                self.drop_buffer_until_flag();
            } else {
                // Search for a valid frame and try to parse the frame.
                if let Some(frame) = self.parse_frame()? {
                    return Ok(Some(frame));
                }
            }

            // Read some bytes into the vacant part of the buffer.
            if 0 == self.stream.read_buf(&mut self.buffer).await? {
                if self.buffer.is_empty() {
                    return Ok(None);
                } else {
                    return Err(std::io::Error::from(ErrorKind::ConnectionReset).into());
                }
            }

            if !self.dropping {
                self.drop_buffer_before_cancel_substitute();
            }
        }
    }

    /// Check for a frame and try to parse it out
    fn parse_frame(&mut self) -> Result<Option<Frame>> {
        let mut buf = Cursor::new(&mut self.buffer[..]);

        match Frame::check(&mut buf) {
            Ok(_) => {
                let len = buf.position() as usize;

                let frame_buf = self.buffer.split_off(len - 2);
                self.buffer.advance(2);
                let frame = Frame::parse(frame_buf)?;

                Ok(Some(frame))
            }
            Err(Error::Incomplete) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Scan the buffer for a Substitute or Cancel byte and drop anything before them.
    fn drop_buffer_before_cancel_substitute(&mut self) {
        if let Some(idx) = self
            .buffer
            .iter()
            .position(|&b| b == SUB_BYTE || b == CANCEL_BYTE)
        {
            self.dropping = self.buffer[idx] == SUB_BYTE;
            self.buffer.advance(idx + 1);
        }
    }

    /// Drop data from the buffer until we see a Flag byte
    fn drop_buffer_until_flag(&mut self) {
        if let Some(idx) = self.buffer.iter().position(|&b| b == FLAG_BYTE) {
            self.buffer.advance(idx + 1);
            self.dropping = false;
        }
    }

    pub async fn write_frame(&mut self, frame: &Frame) -> Result<()> {
        // Allocate a byte slice that is big enough for the unescaped frame
        let mut buf = BytesMut::with_capacity(frame.data_len() + 4);

        // Write the frame data into the byte slice
        frame.serialize(&mut buf);

        // Write the byte slice into the writer
        self.stream.write_all_buf(&mut buf).await?;
        Ok(())
    }

    pub async fn flush(&mut self) -> Result<()> {
        self.stream.flush().await?;
        Ok(())
    }
}
