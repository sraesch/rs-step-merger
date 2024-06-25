use circular::Buffer;
use log::{debug, trace};
use std::{io::Read, sync::Arc};

use crate::{Error, Result};

/// The initial buffer size in bytes.
const BUFFER_SIZE_START: usize = 1024;

/// The buffer growth factor, i.e., the factor by which the buffer size is increased when it is
/// updated.
const BUFFER_GROWTH_FACTOR: usize = 2;

/// A buffered reader that reads from a reader and provides a buffer for efficient reading.
/// The reader is providing the read data as a UTF-8 string.
pub struct BufferedReader<R: Read> {
    /// The source for reading new bytes
    reader: R,

    /// The internal circular buffer to store the read data.
    buffer: Buffer,

    /// The number of valid UTF-8 bytes in the buffer.
    num_valid_utf8_bytes: usize,
}

impl<R: Read> BufferedReader<R> {
    /// Creates a new buffered reader.
    ///
    /// # Arguments
    /// * `reader` - The reader to read from.
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            buffer: Buffer::with_capacity(BUFFER_SIZE_START),
            num_valid_utf8_bytes: 0,
        }
    }

    /// Grows the buffer by the growth factor and fills it with data from the reader.
    pub fn grow(&mut self) -> Result<()> {
        // Grow the buffer capacity by the growth factor.
        let new_capacity = BUFFER_GROWTH_FACTOR * self.buffer.capacity();
        self.buffer.grow(new_capacity);

        debug!("Buffer grown to {} bytes", new_capacity);

        // Fill the buffer with data from the reader.
        self.fill_buffer()
    }

    /// Consumes the buffer up to the given index.
    ///
    /// # Arguments
    /// * `n` - The number of bytes to consume.
    pub fn consumed(&mut self, n: usize) {
        assert!(
            n <= self.num_valid_utf8_bytes,
            "Trying to consume more bytes than available"
        );
        self.num_valid_utf8_bytes -= n;
        self.buffer.consume(n);
    }

    /// Checks if the buffer is already too empty and fills it if necessary.
    pub fn check_if_filled_enough(&mut self) -> Result<()> {
        if self.buffer.available_data() * 4 < self.buffer.capacity() {
            trace!("Buffer too empty, filling it...");

            self.fill_buffer()?;
        }
        Ok(())
    }

    /// Returns as many UTF-8 characters as possible from the buffer.
    pub fn as_str(&self) -> &str {
        // the bytes where we already know that they are valid UTF-8
        let safe_bytes = &self.buffer.data()[..self.num_valid_utf8_bytes];

        unsafe {
            // SAFETY: We know that the bytes in `safe_bytes` are valid UTF-8.
            std::str::from_utf8_unchecked(safe_bytes)
        }
    }

    /// Fills the buffer with data from the reader.
    fn fill_buffer(&mut self) -> Result<()> {
        // get a reference onto the available space and read as many bytes as possible
        let space = self.buffer.space();
        let read = self
            .reader
            .read(space)
            .map_err(|e| Error::IO(Arc::new(e)))?;

        // if we read no bytes, we have reached the end of the input
        if read == 0 {
            return Err(Error::EndOfInput());
        }

        // for the new bytes, we check how many of them are valid UTF-8
        let num_valid_new_utf8_bytes = match std::str::from_utf8(space) {
            Ok(_) => read,
            Err(e) => e.valid_up_to(),
        };
        self.num_valid_utf8_bytes += num_valid_new_utf8_bytes;

        // tell the buffer how many bytes we have read
        self.buffer.fill(read);

        Ok(())
    }
}
