use circular::Buffer;
use std::{io::Read, sync::Arc};
use utf8::{decode, DecodeError};

use crate::{Error, Result};

/// The initial buffer size in bytes.
const BUFFER_SIZE_START: usize = 1024;

/// The buffer growth factor, i.e., the factor by which the buffer size is increased when it is
/// updated.
const BUFFER_GROWTH_FACTOR: usize = 2;

/// A buffered reader that reads from a reader and provides a buffer for efficient reading.
/// The reader is providing the read data as a UTF-8 string.
pub struct BufferedReader<R: Read> {
    reader: R,
    buffer: Buffer,
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
        }
    }

    /// Grows the buffer by the growth factor and fills it with data from the reader.
    pub fn grow(&mut self) -> Result<()> {
        // Grow the buffer capacity by the growth factor.
        let new_capacity = BUFFER_GROWTH_FACTOR * self.buffer.capacity();
        self.buffer.grow(new_capacity);

        // Fill the buffer with data from the reader.
        self.fill_buffer()
    }

    /// Consumes the buffer up to the given index.
    ///
    /// # Arguments
    /// * `n` - The number of bytes to consume.
    pub fn consumed(&mut self, n: usize) {
        self.buffer.consume(n);
    }

    /// Returns as many UTF-8 characters as possible from the buffer.
    pub fn as_str(&self) -> Result<&str> {
        match decode(self.buffer.data()) {
            Ok(s) => Ok(s),
            Err(DecodeError::Incomplete { valid_prefix, .. }) => Ok(valid_prefix),
            Err(err) => {
                panic!("Error: {}", err);
            }
        }
    }

    /// Fills the buffer with data from the reader.
    fn fill_buffer(&mut self) -> Result<()> {
        let read = self
            .reader
            .read(self.buffer.space())
            .map_err(|e| Error::IO(Arc::new(e)))?;

        if read == 0 {
            return Err(Error::EndOfInput());
        } else {
            self.buffer.fill(read);
        }

        Ok(())
    }
}
