use std::io::{BufRead, BufReader, Read};

use crate::Result;

/// A character reader that reads from a buffer.
pub struct CharReader<'a, R: Read> {
    /// The reader to read from.
    reader: BufReader<&'a mut R>,

    /// The characters that are currently in the buffer.
    buffer: Vec<char>,

    /// The position inside the buffer.
    pos: usize,
}

impl<'a, R: Read> CharReader<'a, R> {
    /// Creates a new character reader from a reader.
    ///
    /// # Arguments
    /// * `reader` - The reader to read from.
    pub fn new(reader: &'a mut R) -> Self {
        CharReader {
            reader: BufReader::new(reader),
            buffer: Vec::new(),
            pos: 0,
        }
    }

    /// Reads the next character from the buffer and returns none if the reader is at the end.
    pub fn read(&mut self) -> Result<Option<char>> {
        loop {
            // If there are characters left in the buffer, read them.
            if self.pos < self.buffer.len() {
                let ch = self.buffer[self.pos];
                self.pos += 1;

                return Ok(Some(ch));
            }

            if !self.refresh_buffer()? {
                return Ok(None);
            }
        }
    }

    /// Refreshes the buffer by reading a new line from the reader.
    /// Returns true if a new line was read, false if the reader is at the end.
    fn refresh_buffer(&mut self) -> Result<bool> {
        // read a new line from the reader as characters and store them in the buffer
        self.buffer.clear();
        self.pos = 0;
        let mut line = String::new();
        let bytes_read = self.reader.read_line(&mut line)?;
        if bytes_read == 0 {
            return Ok(false);
        }

        self.buffer.extend(line.chars());

        Ok(true)
    }
}

#[cfg(test)]
mod test {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn test_char_reading() {
        let mut reader = Cursor::new("Hello, World!".as_bytes());
        let mut reader = CharReader::new(&mut reader);
        assert_eq!(reader.read().unwrap(), Some('H'));
        assert_eq!(reader.read().unwrap(), Some('e'));
        assert_eq!(reader.read().unwrap(), Some('l'));
        assert_eq!(reader.read().unwrap(), Some('l'));
        assert_eq!(reader.read().unwrap(), Some('o'));
        assert_eq!(reader.read().unwrap(), Some(','));
        assert_eq!(reader.read().unwrap(), Some(' '));
        assert_eq!(reader.read().unwrap(), Some('W'));
        assert_eq!(reader.read().unwrap(), Some('o'));
        assert_eq!(reader.read().unwrap(), Some('r'));
        assert_eq!(reader.read().unwrap(), Some('l'));
        assert_eq!(reader.read().unwrap(), Some('d'));
        assert_eq!(reader.read().unwrap(), Some('!'));
        assert_eq!(reader.read().unwrap(), None);
    }

    #[test]
    fn test_char_reading_newline() {
        let mut reader = Cursor::new("Hello,\n\nWorld!".as_bytes());
        let mut reader = CharReader::new(&mut reader);
        assert_eq!(reader.read().unwrap(), Some('H'));
        assert_eq!(reader.read().unwrap(), Some('e'));
        assert_eq!(reader.read().unwrap(), Some('l'));
        assert_eq!(reader.read().unwrap(), Some('l'));
        assert_eq!(reader.read().unwrap(), Some('o'));
        assert_eq!(reader.read().unwrap(), Some(','));
        assert_eq!(reader.read().unwrap(), Some('\n'));
        assert_eq!(reader.read().unwrap(), Some('\n'));
        assert_eq!(reader.read().unwrap(), Some('W'));
        assert_eq!(reader.read().unwrap(), Some('o'));
        assert_eq!(reader.read().unwrap(), Some('r'));
        assert_eq!(reader.read().unwrap(), Some('l'));
        assert_eq!(reader.read().unwrap(), Some('d'));
        assert_eq!(reader.read().unwrap(), Some('!'));
        assert_eq!(reader.read().unwrap(), None);
    }
}
