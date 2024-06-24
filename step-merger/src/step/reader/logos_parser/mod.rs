use std::{io::Read, iter::Peekable};

mod buffered_reader;
pub mod lexer_logos;
mod stream_lexer;

use buffered_reader::BufferedReader;
use log::{debug, trace};

use crate::{step::StepEntry, Error, Result};

use self::stream_lexer::{Token, TokenIterator};

use super::STEPReaderTrait;

/// A peekable token iterator that allows to peek the next token without consuming it.
type PTokenIterator<'a> = Peekable<TokenIterator<'a>>;

/// The STEP reader consumes a reader and parses the STEP entries from it. All entries are returned
/// as `StepEntry` instances in the order they appear in the file.
/// The reader implements the `Iterator` trait and returns `Result<StepEntry>` instances s.t. the
/// returned entries can be processed in a streaming fashion.
pub struct STEPReader<R: Read> {
    /// The internal buffer reader that reads from the input reader.
    reader: BufferedReader<R>,

    /// Indicates if the end of the data section has been reached.
    reached_end: bool,
}

impl<R: Read> STEPReader<R> {
    /// Tries to parse an element from the reader with the given token based parser `p`.
    /// The parsers tries
    ///
    /// # Arguments
    /// * `p` - The parser that tries to parse the element from the reader.
    ///         The parser returns None if the element could not be parsed and Some(T) if the
    ///         element was successfully parsed.
    fn parse_element<P, T>(&mut self, p: P) -> Result<T>
    where
        P: FnMut(&mut PTokenIterator) -> Result<T>,
    {
        let mut p = p;

        // Check if the buffer is filled enough and if not, fill it.
        // We ignore if the end of the file is reached.
        match self.reader.check_if_filled_enough() {
            Err(Error::EndOfInput()) => {}
            Err(err) => return Err(err),
            Ok(()) => {}
        }

        // try yo parse until it works or the end of the file is reached
        loop {
            let lexer = TokenIterator::new(self.reader.as_str()?);
            let consumed_bytes = lexer.consumed_bytes();
            let mut lexer = lexer.peekable();

            // try to parse the element and if it is successful, consume the bytes and return
            if let Ok(ret) = p(&mut lexer) {
                // consume the bytes that have been successfully parsed
                self.reader
                    .consumed(consumed_bytes.load(std::sync::atomic::Ordering::Relaxed));

                return Ok(ret);
            }

            // try to grow the buffer and retry
            self.reader.grow()?;
        }
    }
}

impl<R: Read> STEPReader<R> {
    /// Parses the initial ISO String 'ISO-10303-21' and fails if it is not found or not correctly
    /// formatted.
    fn parse_iso_line(&mut self) -> Result<()> {
        debug!("Parsing ISO line");
        self.parse_element(|p| {
            match p.skip_whitespace_tokens() {
                Ok(()) => {}
                Err(err) => return Err(err),
            }

            match p.next() {
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

            p.skip_whitespace_tokens()?;
            match p.next() {
                Some(Ok(Token::Sem)) => {}
                Some(Ok(token)) => {
                    return Err(Error::UnexpectedToken(";".to_string(), token.to_string()))
                }
                Some(Err(err)) => return Err(err),
                None => return Err(Error::EndOfInput()),
            }

            Ok(Some(()))
        })?;

        Ok(())
    }

    /// Searches for the DATA section and fails if it is not found.
    fn find_data_section(&mut self) -> Result<()> {
        debug!("Searching for DATA section");
        self.parse_element(|p| {
            loop {
                match p.next() {
                    Some(Ok(Token::Data)) => break,
                    Some(Ok(_)) => {}
                    Some(Err(err)) => return Err(err),
                    None => return Err(Error::NoDataSection()),
                }
            }

            p.skip_whitespace_tokens()?;

            match p.next() {
                Some(Ok(Token::Sem)) => Ok(()),
                Some(Ok(token)) => Err(Error::UnexpectedToken(";".to_string(), token.to_string())),
                Some(Err(err)) => Err(err),
                None => Err(Error::EndOfInput()),
            }
        })
    }

    /// Reads the next STEP entry and returns none if the end of the section is reached.
    /// Otherwise, returns the read STEP entry or an error if the input is invalid.
    fn read_next_entry(&mut self) -> Result<Option<StepEntry>> {
        trace!("Reading next entry");

        // check if the end of the section is already reached
        if self.reached_end {
            return Ok(None);
        }

        let mut reached_end = false;
        let ret = self.parse_element(|p| {
            p.skip_whitespace_tokens()?;

            // expect reference to the next STEP entry or the end of the section
            let id = match p.next() {
                Some(Ok(Token::Reference(id))) => id,
                Some(Ok(Token::Endsec)) => {
                    reached_end = true;
                    return Ok(None);
                }
                Some(Ok(token)) => {
                    println!("{:?}", token);
                    println!("{:?}", p.next());
                    println!("{:?}", p.next());

                    return Err(Error::UnexpectedToken("#".to_string(), token.to_string()));
                }
                Some(Err(err)) => return Err(err),
                None => return Err(Error::EndOfInput()),
            };

            p.skip_whitespace_tokens()?;

            // expect equal sign
            match p.next() {
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
                match p.next() {
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
        })?;

        self.reached_end = reached_end;

        Ok(ret)
    }
}

trait BasicParserFunctionalities {
    /// Skips whitespace tokens, i.e., whitespace and comments.
    fn skip_whitespace_tokens(&mut self) -> Result<()>;
}

impl<'a> BasicParserFunctionalities for PTokenIterator<'a> {
    fn skip_whitespace_tokens(&mut self) -> Result<()> {
        loop {
            match self.peek() {
                Some(Ok(Token::Whitespace)) => {
                    if let Some(Err(err)) = self.next() {
                        return Err(err);
                    }
                }
                Some(Ok(Token::Comments)) => {
                    if let Some(Err(err)) = self.next() {
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
        let reader = BufferedReader::new(reader);

        let mut step_parser = STEPReader {
            reader,
            reached_end: false,
        };

        step_parser.parse_iso_line()?;
        step_parser.find_data_section()?;

        Ok(step_parser)
    }
}
