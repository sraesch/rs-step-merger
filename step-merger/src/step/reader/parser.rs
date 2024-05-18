use std::{io::Read, iter::Peekable};

use super::{
    char_reader::CharReader,
    tokenizer::{Token, Tokenizer},
};

use crate::{Error, Result};

/// A parser that provides simple parsing operations like skipping sequences, reading strings,
/// numbers, etc.
pub struct Parser<R: Read> {
    /// The tokenizer used to parse the input.
    tokenizer: Peekable<Tokenizer<CharReader<R>>>,
}

impl<R: Read> Parser<R> {
    /// Creates a new parser from the given character reader.
    ///
    /// # Arguments
    /// * `reader` - The character reader to tokenize.
    pub fn new(reader: R) -> Self {
        Parser {
            tokenizer: Tokenizer::new(CharReader::new(reader)).peekable(),
        }
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
                Some(Ok(Token::Comment)) => {
                    if let Some(Err(err)) = self.tokenizer.next() {
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
            match self.tokenizer.peek() {
                Some(Ok(Token::Character(ch))) => {
                    if !predicate(*ch) {
                        return Ok(count);
                    }

                    if let Some(Err(err)) = self.tokenizer.next() {
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
            match self.tokenizer.peek() {
                Some(Ok(Token::Character(ch))) => {
                    if !predicate(*ch) {
                        return Ok(result);
                    }

                    result.push(*ch);
                    if let Some(Err(err)) = self.tokenizer.next() {
                        return Err(err);
                    }
                }
                Some(Ok(Token::Whitespace)) => {
                    if !ignore_whitespace {
                        return Ok(result);
                    } else {
                        result.push(' ');
                        if let Some(Err(err)) = self.tokenizer.next() {
                            return Err(err);
                        }
                    }
                }
                Some(Ok(Token::Comment)) => {
                    if !ignore_whitespace {
                        return Ok(result);
                    } else {
                        result.push(' ');
                        if let Some(Err(err)) = self.tokenizer.next() {
                            return Err(err);
                        }
                    }
                }
                Some(Err(err)) => return Err(Error::FailedSequence(Box::new(err.clone()))),
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
            match self.tokenizer.next() {
                Some(Ok(Token::Character(parsed_ch))) => {
                    if parsed_ch != ch {
                        return Err(Error::UnexpectedToken(
                            ch.to_string(),
                            parsed_ch.to_string(),
                        ));
                    }
                }
                Some(Ok(Token::Whitespace)) => {
                    return Err(Error::UnexpectedToken(
                        sequence.to_string(),
                        "whitespace".to_string(),
                    ));
                }
                Some(Ok(Token::Comment)) => {
                    return Err(Error::UnexpectedToken(
                        sequence.to_string(),
                        "comment.".to_string(),
                    ));
                }
                Some(Err(err)) => return Err(Error::FailedSequence(Box::new(err))),
                None => {
                    return Err(Error::UnexpectedToken(
                        sequence.to_string(),
                        "eof".to_string(),
                    ))
                }
            }
        }

        Ok(())
    }

    /// Reads a u64 number.
    pub fn read_u64(&mut self) -> Result<u64> {
        self.skip_whitespace_tokens()?;
        let s = self.read_string(|ch| ch.is_ascii_digit(), false)?;
        s.parse().map_err(|_| Error::InvalidNumber(s))
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
