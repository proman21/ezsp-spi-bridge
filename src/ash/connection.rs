use std::io::{BufWriter, Cursor, Read, Write};

use bytes::{Buf, BytesMut};

use super::{
    error::{Error, Result},
    frame::{Frame, FrameFormat, CANCEL_BYTE, FLAG_BYTE, SUB_BYTE},
};
use crate::buffer::Buffer;

/// A wrapper around a reader and writer that reads and writes ASH frames
pub struct Connection<'a, R: Read, W: Write> {
    buffer: Buffer<'a>,
    writer: BufWriter<W>,
    reader: R,
    dropping: bool,
}

impl<'a, R: Read, W: Write> Connection<'a, R, W> {
    pub fn new(reader: R, writer: W) -> Connection<'a, R, W> {
        Self {
            buffer: BytesMut::with_capacity(2048).into(),
            writer: BufWriter::with_capacity(2048, writer),
            reader,
            dropping: false,
        }
    }

    pub fn into_inner(self) -> std::result::Result<(R, W), (Self, std::io::Error)> {
        let Self {
            buffer,
            writer,
            reader,
            dropping,
        } = self;
        match writer.into_inner() {
            Ok(w) => Ok((reader, w)),
            Err(e) => {
                let (err, w) = e.into_parts();

                Err((
                    Self {
                        writer: w,
                        reader,
                        buffer,
                        dropping,
                    },
                    err,
                ))
            }
        }
    }

    /// Attempt to read a frame from the internal buffer, filling it with more
    /// data from the inner reader if none can be found
    pub fn read_frame(&mut self) -> Result<Option<Frame>> {
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
            self.buffer.fill_from_reader(&mut self.reader)?;

            self.drop_buffer_before_cancel_substitute();
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

    pub fn write_frame(&mut self, frame: &Frame) -> Result<()> {
        // Allocate a byte slice that is big enough for the unescaped frame
        let mut buf = BytesMut::with_capacity(frame.data_len() + 4);

        // Write the frame data into the byte slice
        frame.serialize(&mut buf);

        // Write the byte slice into the writer
        self.writer.write_all(&buf)?;
        Ok(())
    }

    pub fn flush(&mut self) -> Result<()> {
        self.writer.flush()?;
        Ok(())
    }
}
