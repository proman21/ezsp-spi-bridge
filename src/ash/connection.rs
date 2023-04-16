use std::{
    io::{BufWriter, Cursor, Read, Write},
    slice::from_raw_parts_mut,
};

use bytes::{Buf, BytesMut};

use super::{
    error::{Error, Result},
    frame::{Frame, FrameFormat, CANCEL_BYTE, FLAG_BYTE, SUB_BYTE},
};

/// A wrapper around a reader and writer that reads and writes ASH frames
pub struct Connection<R: Read, W: Write> {
    buffer: BytesMut,
    writer: BufWriter<W>,
    reader: R,
    dropping: bool,
}

impl<R: Read, W: Write> Connection<R, W> {
    pub fn new(reader: R, writer: W) -> Connection<R, W> {
        Self {
            buffer: BytesMut::with_capacity(2048),
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
                // If we have received a Substitute byte, drop data until we see a Flag byte
                if let Some(idx) = self.buffer.iter().position(|&b| b == FLAG_BYTE) {
                    self.buffer.advance(idx + 1);
                    self.dropping = false;
                }
            } else {
                // Search for a valid frame and try to parse the frame.
                if let Some(frame) = self.parse_frame()? {
                    return Ok(Some(frame));
                }
            }

            // Read some bytes into the vacant part of the buffer.
            let spare_cap = self.buffer.spare_capacity_mut();
            let read_buf =
                unsafe { from_raw_parts_mut(spare_cap.as_mut_ptr() as *mut u8, spare_cap.len()) };

            let read = self.reader.read(read_buf)?;

            unsafe { self.buffer.set_len(self.buffer.len() + read) }

            // Scan the buffer for a Substitute or Cancel byte and drop anything before them.
            if let Some(idx) = self
                .buffer
                .iter()
                .position(|&b| b == SUB_BYTE || b == CANCEL_BYTE)
            {
                self.dropping = self.buffer[idx] == SUB_BYTE;
                self.buffer.advance(idx + 1);
            }
        }
    }

    /// Check for a frame and try to parse it out
    fn parse_frame(&mut self) -> Result<Option<Frame>> {
        let mut buf = Cursor::new(&mut self.buffer[..]);

        match Frame::check(&mut buf) {
            Ok(_) => {
                let len = buf.position() as usize;
                buf.set_position(0);

                let frame = Frame::parse(&buf.get_ref()[..len - 2])?;
                self.buffer.advance(len);

                Ok(Some(frame))
            }
            Err(Error::Incomplete) => Ok(None),
            Err(e) => Err(e),
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
