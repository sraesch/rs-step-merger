use std::{io::Read, iter::Peekable};

pub mod lexer_logos;
mod stream_lexer;

use crate::{step::StepEntry, Error, Result};

use self::stream_lexer::{Token, TokenIterator};

use super::STEPReaderTrait;

// pub mod lexer_logos;

/// The STEP reader consumes a reader and parses the STEP entries from it. All entries are returned
/// as `StepEntry` instances in the order they appear in the file.
/// The reader implements the `Iterator` trait and returns `Result<StepEntry>` instances s.t. the
/// returned entries can be processed in a streaming fashion.
pub struct STEPReader<R: Read> {
    tokenizer: Peekable<TokenIterator<R>>,

    /// Indicates if the end of the data section has been reached.
    reached_end: bool,
}

impl<R: Read> STEPReader<R> {
    /// Parses the initial ISO String 'ISO-10303-21' and fails if it is not found or not correctly
    /// formatted.
    fn parse_iso_line(&mut self) -> Result<()> {
        self.skip_whitespace_tokens()?;
        match self.tokenizer.next() {
            Some(Ok(Token::StartTag)) => {}
            Some(Ok(token)) => {
                return Err(Error::UnexpectedToken(
                    "ISO-10303-21".to_string(),
                    token.to_string(),
                ))
            }
            Some(Err(err)) => return Err(err),
            None => return Err(Error::EndOfInput()),
        }

        self.skip_whitespace_tokens()?;
        match self.tokenizer.next() {
            Some(Ok(Token::Sem)) => {}
            Some(Ok(token)) => {
                return Err(Error::UnexpectedToken(";".to_string(), token.to_string()))
            }
            Some(Err(err)) => return Err(err),
            None => return Err(Error::EndOfInput()),
        }

        Ok(())
    }

    /// Searches for the DATA section and fails if it is not found.
    fn find_data_section(&mut self) -> Result<()> {
        loop {
            match self.tokenizer.next() {
                Some(Ok(Token::Data)) => break,
                Some(Ok(_)) => {}
                Some(Err(err)) => return Err(err),
                None => return Err(Error::NoDataSection()),
            }
        }

        self.skip_whitespace_tokens()?;

        match self.tokenizer.next() {
            Some(Ok(Token::Sem)) => Ok(()),
            Some(Ok(token)) => Err(Error::UnexpectedToken(";".to_string(), token.to_string())),
            Some(Err(err)) => Err(err),
            None => Err(Error::EndOfInput()),
        }
    }

    /// Reads the next STEP entry and returns none if the end of the section is reached.
    /// Otherwise, returns the read STEP entry or an error if the input is invalid.
    fn read_next_entry(&mut self) -> Result<Option<StepEntry>> {
        // check if the end of the section is already reached
        if self.reached_end {
            return Ok(None);
        }

        self.skip_whitespace_tokens()?;

        // expect reference to the next STEP entry or the end of the section
        let id = match self.tokenizer.next() {
            Some(Ok(Token::Reference(id))) => id,
            Some(Ok(Token::Endsec)) => {
                self.reached_end = true;
                return Ok(None);
            }
            Some(Ok(token)) => {
                return Err(Error::UnexpectedToken("#".to_string(), token.to_string()))
            }
            Some(Err(err)) => return Err(err),
            None => return Err(Error::EndOfInput()),
        };

        self.skip_whitespace_tokens()?;

        // expect equal sign
        match self.tokenizer.next() {
            Some(Ok(Token::Eq)) => {}
            Some(Ok(token)) => {
                return Err(Error::UnexpectedToken("=".to_string(), token.to_string()))
            }
            Some(Err(err)) => return Err(err),
            None => return Err(Error::EndOfInput()),
        }

        // parse the definition of the STEP entry
        let mut definition = String::new();
        loop {
            match self.tokenizer.next() {
                Some(Ok(Token::Sem)) => break,
                Some(Ok(Token::Whitespace)) => definition.push(' '),
                Some(Ok(Token::Comments)) => {}
                Some(Ok(Token::Definition(d))) => {
                    definition.push_str(&d);
                }
                Some(Ok(Token::Eq)) => definition.push('='),
                Some(Ok(Token::String(s))) => {
                    definition.push('\'');
                    definition.push_str(&s);
                    definition.push('\'');
                }
                Some(Ok(Token::Reference(r))) => definition.push_str(&format!("#{}", r)),
                Some(Ok(token)) => {
                    return Err(Error::UnexpectedToken(";".to_string(), token.to_string()))
                }
                Some(Err(err)) => return Err(err),
                None => return Err(Error::EndOfInput()),
            }
        }

        Ok(Some(StepEntry { id, definition }))
    }

    /// Skips whitespace tokens, i.e., whitespace and comments.
    pub fn skip_whitespace_tokens(&mut self) -> Result<()> {
        loop {
            match self.tokenizer.peek() {
                Some(Ok(Token::Whitespace)) => {
                    if let Some(Err(err)) = self.tokenizer.next() {
                        return Err(err);
                    }
                }
                Some(Ok(Token::Comments)) => {
                    if let Some(Err(err)) = self.tokenizer.next() {
                        return Err(err);
                    }
                }
                _ => return Ok(()),
            }
        }
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
        "Logos STEP reader"
    }

    fn new(reader: R) -> Result<Self> {
        let tokenizer = TokenIterator::new(reader).peekable();

        let mut step_parser = STEPReader {
            tokenizer,
            reached_end: false,
        };

        step_parser.parse_iso_line()?;
        step_parser.find_data_section()?;

        Ok(step_parser)
    }
}
