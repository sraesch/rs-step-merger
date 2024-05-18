use std::io::{BufRead, BufReader, Read};

use crate::Result;

/// A character reader that reads from a buffer.
pub struct CharReader<R: Read> {
    /// The reader to read from.
    reader: BufReader<R>,

    /// The characters that are currently in the buffer.
    buffer: Vec<char>,

    /// The position inside the buffer.
    pos: usize,
}

impl<R: Read> CharReader<R> {
    /// Creates a new character reader from a reader.
    ///
    /// # Arguments
    /// * `reader` - The reader to read from.
    pub fn new(reader: R) -> Self {
        CharReader {
            reader: BufReader::new(reader),
            buffer: Vec::new(),
            pos: 0,
        }
    }

    /// Refreshes the buffer by reading a new line from the reader.
    /// Returns true if a new line was read, false if the reader is at the end.
    fn refresh_buffer(&mut self) -> Result<bool> {
        // make sure the buffer is empty
        assert!(self.pos == self.buffer.len(), "Buffer is not empty");

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

impl<R: Read> Iterator for CharReader<R> {
    type Item = Result<char>;

    fn next(&mut self) -> Option<Result<char>> {
        loop {
            // If there are characters left in the buffer, read them.
            if self.pos < self.buffer.len() {
                let ch = self.buffer[self.pos];
                self.pos += 1;

                return Some(Ok(ch));
            }

            match self.refresh_buffer() {
                Ok(true) => {}
                Ok(false) => return None,
                Err(err) => return Some(Err(err)),
            }
        }
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
        assert_eq!(reader.next().unwrap().unwrap(), 'H');
        assert_eq!(reader.next().unwrap().unwrap(), 'e');
        assert_eq!(reader.next().unwrap().unwrap(), 'l');
        assert_eq!(reader.next().unwrap().unwrap(), 'l');
        assert_eq!(reader.next().unwrap().unwrap(), 'o');
        assert_eq!(reader.next().unwrap().unwrap(), ',');
        assert_eq!(reader.next().unwrap().unwrap(), ' ');
        assert_eq!(reader.next().unwrap().unwrap(), 'W');
        assert_eq!(reader.next().unwrap().unwrap(), 'o');
        assert_eq!(reader.next().unwrap().unwrap(), 'r');
        assert_eq!(reader.next().unwrap().unwrap(), 'l');
        assert_eq!(reader.next().unwrap().unwrap(), 'd');
        assert_eq!(reader.next().unwrap().unwrap(), '!');
        assert!(reader.next().is_none());
    }

    #[test]
    fn test_char_reading_newline() {
        let mut reader = Cursor::new("Hello,\n\nWorld!".as_bytes());
        let mut reader = CharReader::new(&mut reader);
        assert_eq!(reader.next().unwrap().unwrap(), 'H');
        assert_eq!(reader.next().unwrap().unwrap(), 'e');
        assert_eq!(reader.next().unwrap().unwrap(), 'l');
        assert_eq!(reader.next().unwrap().unwrap(), 'l');
        assert_eq!(reader.next().unwrap().unwrap(), 'o');
        assert_eq!(reader.next().unwrap().unwrap(), ',');
        assert_eq!(reader.next().unwrap().unwrap(), '\n');
        assert_eq!(reader.next().unwrap().unwrap(), '\n');
        assert_eq!(reader.next().unwrap().unwrap(), 'W');
        assert_eq!(reader.next().unwrap().unwrap(), 'o');
        assert_eq!(reader.next().unwrap().unwrap(), 'r');
        assert_eq!(reader.next().unwrap().unwrap(), 'l');
        assert_eq!(reader.next().unwrap().unwrap(), 'd');
        assert_eq!(reader.next().unwrap().unwrap(), '!');
        assert!(reader.next().is_none());
    }
}
