use std::io::{BufRead, BufReader, Read};

use log::trace;

use crate::{Error, Result};

use super::StepData;



/// A reader for a STEP file.
pub struct STEPReader<R: Read> {
    /// The lines of the STEP file.
    reader: BufReader<R>,

    /// The ISO string of the STEP file.
    iso: String,
}

impl<R: Read> STEPReader<R> {
    pub fn new(reader: R) -> STEPReader<R> {
        let reader = BufReader::new(reader);

        STEPReader {
            reader,
            iso: String::new(),
        }
    }

    /// Reads the STEP file and returns the step data.
    pub fn read(mut self) -> Result<StepData> {
        self.read_header()?;
        Ok(self.into_step_data())
    }

    /// Transforms the reader into the step data.
    fn into_step_data(self) -> StepData {
        StepData::new(self.iso)
    }

    /// Reads the header of the STEP file.
    fn read_header(&mut self) -> Result<()> {
        // read iso string
        let iso: String = self.read_next_line()?.trim().to_owned();
        if !iso.starts_with("ISO") || !iso.ends_with(';') {
            return Err(Error::IO(format!("Invalid ISO string, got '{}'", iso)));
        }

        self.iso = iso[..iso.len() - 1].to_owned();
        trace!("ISO string: {}", self.iso);

        self.seek_next_line_entry("HEADER;")?;
        self.seek_until_word("FILE_DESCRIPTION")?;
        self.seek_until_char('(')?;
        self.seek_until_char('(')?;
        self.seek_until_char(')')?;
        self.seek_until_char(',')?;
        self.seek_until_char('\'')?;
        let implementation_level = self.copy_until_char('\'')?;
        trace!("Implementation level: {}", implementation_level);

        Ok(())
    }

    /// Seeks until the given character has been found and skips over it.
    ///
    /// # Arguments
    /// * `chr` - The character to seek.
    fn seek_until_char(&mut self, chr: char) -> Result<()> {
        // find the character
        for c in self.reader.chars() {
            let c = c.map_err(|e| Error::IO(format!("Failed to read char: {}", e)))?;
            if c == chr {
                break;
            }
        }

        self.reader.chars().next();

        Ok(())
    }

    /// Seeks until the given character has been found and skips over it.
    ///
    /// # Arguments
    /// * `chr` - The character to seek.
    fn copy_until_char(&mut self, chr: char) -> Result<String> {
        let mut buf: String = String::new();

        // find the character
        let chars = self.reader.chars();
        for c in chars {
            let c = c.map_err(|e| Error::IO(format!("Failed to read char: {}", e)))?;
            if c == chr {
                break;
            } else {
                buf.push(c);
            }
        }

        Ok(buf)
    }

    /// Seeks until the given word has been found and skips over it.
    ///
    /// # Arguments
    /// * `word` - The word to seek.
    fn seek_until_word(&mut self, word: &str) -> Result<()> {
        // find the word
        let chars = self.reader.chars();

        let mut word_chars = word.chars().peekable();
        for c in chars {
            let c = c.map_err(|e| Error::IO(format!("Failed to read char: {}", e)))?;
            if let Some(w) = word_chars.next() {
                // reset word chars if the character does not match
                if c != w {
                    word_chars = word.chars().peekable();
                }
            } else {
                break;
            }

            if word_chars.peek().is_none() {
                break;
            }
        }

        Ok(())
    }

    /// Seeks the next line which consists of the given keyword.
    ///
    /// # Arguments
    /// * `keyword` - The keyword to seek.
    fn seek_next_line_entry(&mut self, keyword: &str) -> Result<()> {
        // search for keyword
        loop {
            let line = self.read_next_line()?;
            if line.trim() == keyword {
                break;
            }
        }

        Ok(())
    }

    /// Reads the next line from the STEP file.
    /// Also increments the line index.
    #[inline]
    fn read_next_line(&mut self) -> Result<String> {
        let mut line = String::new();
        let ret = self
            .reader
            .read_line(&mut line)
            .map_err(|e| Error::IO(format!("Failed to read line: {}", e)))?;

        if ret == 0 {
            return Err(Error::IO("End of file reached".to_owned()));
        }

        // map none to error
        Ok(line)
    }
}
