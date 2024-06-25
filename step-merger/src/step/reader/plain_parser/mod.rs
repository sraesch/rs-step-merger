use std::io::Read;

use self::parser::Parser;

use crate::{step::StepEntry, Error, Result};

use super::STEPReaderTrait;

mod char_reader;
mod parser;
mod tokenizer;

/// The STEP reader consumes a reader and parses the STEP entries from it. All entries are returned
/// as `StepEntry` instances in the order they appear in the file.
/// The reader implements the `Iterator` trait and returns `Result<StepEntry>` instances s.t. the
/// returned entries can be processed in a streaming fashion.
pub struct STEPReader<R: Read> {
    /// The parser used to parse the STEP entries.
    parser: Parser<R>,

    /// Indicates if the end of the data section has been reached.
    reached_end: bool,
}

impl<R: Read> STEPReader<R> {
    /// Parses the initial ISO String 'ISO-10303-21' and fails if it is not found or not correctly
    /// formatted.
    fn parse_iso_line(&mut self) -> Result<()> {
        self.parser.skip_whitespace_tokens()?;
        self.parser.read_exact_sequence("ISO-10303-21")?;
        self.parser.skip_whitespace_tokens()?;
        self.parser.read_exact_sequence(";")?;

        Ok(())
    }

    /// Searches for the DATA section and fails if it is not found.
    fn find_data_section(&mut self) -> Result<()> {
        loop {
            self.parser.skip_whitespace_tokens()?;
            let identifier = self
                .parser
                .read_string(|ch| ch.is_ascii_alphabetic(), false)?;

            if identifier == "DATA" {
                self.parser.skip_whitespace_tokens()?;
                self.parser.read_exact_sequence(";")?;

                break;
            }

            let num_skipped = self.parser.skip_until(|ch| !ch.is_ascii_alphabetic())?;
            if num_skipped == 0 && identifier.is_empty() {
                return Err(Error::NoDataSection());
            }
        }

        Ok(())
    }

    /// Reads the next STEP entry and returns none if the end of the section is reached.
    /// Otherwise, returns the read STEP entry or an error if the input is invalid.
    fn read_next_entry(&mut self) -> Result<Option<StepEntry>> {
        // check if the end of the section is already reached
        if self.reached_end {
            return Ok(None);
        }

        self.parser.skip_whitespace_tokens()?;

        // check if there is some identifier
        let identifier = self
            .parser
            .read_string(|ch| ch.is_ascii_alphabetic(), false)?;

        // check if the read identifier is the end of the section
        if identifier == "ENDSEC" {
            self.reached_end = true;
            self.parser.skip_whitespace_tokens()?;
            self.parser.read_exact_sequence(";")?;

            return Ok(None);
        } else if !identifier.is_empty() {
            return Err(Error::UnexpectedIdentifier(identifier));
        }

        // no identifier, so there must be a new STEP entry
        self.parser.read_exact_sequence("#")?;
        let id = self.parser.read_u64()?;
        self.parser.skip_whitespace_tokens()?;
        self.parser.read_exact_sequence("=")?;
        self.parser.skip_whitespace_tokens()?;
        let definition = self.parser.read_string(|ch| ch != ';', true)?;
        self.parser.read_exact_sequence(";")?;

        Ok(Some(StepEntry { id, definition }))
    }
}

impl<R: Read> Iterator for STEPReader<R> {
    type Item = Result<StepEntry>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.read_next_entry() {
            Ok(Some(entry)) => Some(Ok(entry)),
            Ok(None) => None,
            Err(err) => Some(Err(err)),
        }
    }
}

impl<R: Read> STEPReaderTrait<R> for STEPReader<R> {
    fn get_name(&self) -> &'static str {
        "Plain STEP reader"
    }

    fn new(reader: R) -> Result<Self> {
        let mut step_parser = STEPReader {
            parser: Parser::new(reader),
            reached_end: false,
        };

        step_parser.parse_iso_line()?;
        step_parser.find_data_section()?;

        Ok(step_parser)
    }
}
