use std::{io::Read, iter::Peekable};

use super::{
    char_parser::CharReader,
    whitespace_parser::{Token, WhitespaceParser},
};

use crate::{Error, Result};

/// A parser used by the STEP parser
pub struct Parser<R: Read> {
    reader: Peekable<WhitespaceParser<CharReader<R>>>,
}

impl<R: Read> Parser<R> {
    /// Creates a new STEP parser from a reader.
    ///
    /// # Arguments
    /// * `reader` - The reader to read from.
    pub fn new(reader: R) -> Self {
        Parser {
            reader: WhitespaceParser::new(CharReader::new(reader)).peekable(),
        }
    }

    /// Skips whitespace tokens.
    pub fn skip_whitespace_tokens(&mut self) -> Result<()> {
        loop {
            match self.reader.peek() {
                Some(Ok(Token::Whitespace)) => {
                    if let Some(Err(err)) = self.reader.next() {
                        return Err(err);
                    }
                }
                Some(Ok(Token::Comment)) => {
                    if let Some(Err(err)) = self.reader.next() {
                        return Err(err);
                    }
                }
                _ => return Ok(()),
            }
        }
    }

    /// Skips until the predicate is not true anymore.
    ///
    /// # Arguments
    /// * `predicate` - The predicate to check for.
    pub fn skip_until(&mut self, predicate: impl Fn(char) -> bool) -> Result<usize> {
        let mut count: usize = 0;

        loop {
            match self.reader.peek() {
                Some(Ok(Token::Character(ch))) => {
                    if !predicate(*ch) {
                        return Ok(count);
                    }

                    if let Some(Err(err)) = self.reader.next() {
                        return Err(err);
                    } else {
                        count += 1;
                    }
                }
                _ => return Ok(count),
            }
        }
    }

    /// Reads as long as the predicate is true.
    /// Returns an error if the end of the input is reached.
    /// Otherwise, returns the read string.
    ///
    /// # Arguments
    /// * `predicate` - The predicate to check for.
    /// * `ignore_whitespace` - Whether to ignore whitespace and comments. If encountered, a space
    ///                         is being added.
    pub fn read_string(
        &mut self,
        predicate: impl Fn(char) -> bool,
        ignore_whitespace: bool,
    ) -> Result<String> {
        let mut result = String::new();

        loop {
            match self.reader.peek() {
                Some(Ok(Token::Character(ch))) => {
                    if !predicate(*ch) {
                        return Ok(result);
                    }

                    result.push(*ch);
                    if let Some(Err(err)) = self.reader.next() {
                        return Err(err);
                    }
                }
                Some(Ok(Token::Whitespace)) => {
                    if !ignore_whitespace {
                        return Ok(result);
                    } else {
                        result.push(' ');
                        if let Some(Err(err)) = self.reader.next() {
                            return Err(err);
                        }
                    }
                }
                Some(Ok(Token::Comment)) => {
                    if !ignore_whitespace {
                        return Ok(result);
                    } else {
                        result.push(' ');
                        if let Some(Err(err)) = self.reader.next() {
                            return Err(err);
                        }
                    }
                }
                Some(Err(err)) => return Err(err.clone()),
                None => return Ok(result),
            }
        }
    }

    /// Reads a sequence of characters and checks if it matches the given sequence.
    ///
    /// # Arguments
    /// * `sequence` - The sequence to check for.
    pub fn read_exact_sequence(&mut self, sequence: &str) -> Result<()> {
        for ch in sequence.chars() {
            match self.reader.next() {
                Some(Ok(Token::Character(parsed_ch))) => {
                    if parsed_ch != ch {
                        return Err(Error::InvalidFormat(format!(
                            "Expected '{}', got '{}'.",
                            ch, parsed_ch
                        )));
                    }
                }
                Some(Ok(Token::Whitespace)) => {
                    return Err(Error::InvalidFormat(format!(
                        "Expected '{}', got whitespace.",
                        sequence
                    )));
                }
                Some(Ok(Token::Comment)) => {
                    return Err(Error::InvalidFormat(format!(
                        "Expected '{}', got comment.",
                        sequence
                    )));
                }
                Some(Err(err)) => return Err(err.clone()),
                None => {
                    return Err(Error::InvalidFormat(format!(
                        "Expected '{}', got eof.",
                        sequence
                    )))
                }
            }
        }

        Ok(())
    }

    /// Reads a u64 number.
    pub fn read_u64(&mut self) -> Result<u64> {
        self.skip_whitespace_tokens()?;
        let s = self.read_string(|ch| ch.is_ascii_digit(), false)?;
        s.parse()
            .map_err(|_| Error::InvalidFormat("Invalid number".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn test_read_until() {
        let mut input = Cursor::new("abc-def");
        let mut reader = Parser::new(&mut input);

        let result = reader.read_string(|ch| ch != '-', false).unwrap();
        assert_eq!(result, "abc".to_string());

        let result = reader.read_string(|ch| ch == '-', false).unwrap();
        assert_eq!(result, "-".to_string());

        let result = reader.read_string(|ch| ch != '-', false).unwrap();
        assert_eq!(result, "def".to_string());
    }

    #[test]
    fn test_read_exact_sequence1() {
        let mut input = Cursor::new("abc-def");
        let mut reader = Parser::new(&mut input);

        reader.read_exact_sequence("abc").unwrap();
        reader.read_exact_sequence("-").unwrap();
        reader.read_exact_sequence("def").unwrap();
    }

    #[test]
    fn test_read_exact_sequence2() {
        let mut input = Cursor::new("abc-def");
        let mut reader = Parser::new(&mut input);

        let result = reader.read_exact_sequence("abc");
        assert!(result.is_ok());

        let result = reader.read_exact_sequence("-");
        assert!(result.is_ok());

        let result = reader.read_exact_sequence("def2");
        assert!(result.is_err());
    }

    #[test]
    fn test_skip_until() {
        let mut input = Cursor::new("abc-def");
        let mut reader = Parser::new(&mut input);

        let n = reader.skip_until(|ch| ch != 'c').unwrap();
        assert_eq!(n, 2);

        let s = reader.read_string(|_| true, true).unwrap();

        assert_eq!(s, "c-def".to_string());
    }

    #[test]
    fn test_read_u64() {
        let mut input = Cursor::new("123 456");
        let mut reader = Parser::new(&mut input);

        let n = reader.read_u64().unwrap();
        assert_eq!(n, 123);

        let n = reader.read_u64().unwrap();
        assert_eq!(n, 456);
    }
}
