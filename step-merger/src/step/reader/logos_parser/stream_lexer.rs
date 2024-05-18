use std::{fmt::Display, io::Read, sync::Arc};

use logos::Logos;
use utf8::{decode, DecodeError};

use crate::{Error, Result};

use circular::Buffer;

const BUFFER_SIZE_START: usize = 1024;
const BUFFER_GROWTH_FACTOR: usize = 2;

#[derive(Logos, Debug, PartialEq)]
pub enum Token {
    #[regex(r"\/\*([^*]|\*[^\/])*\*\/", logos::skip)]
    Comments,
    #[regex(r"[ \t\r\n\f]+", logos::skip)]
    Whitespace,
    #[token("=")]
    Eq,
    #[token(";")]
    Sem,
    #[regex(r"[#][\s]*[1-9][0-9]*", |lex| lex.slice()[1..].trim_start().parse::<u64>().unwrap())]
    Reference(u64),
    #[token("HEADER")]
    Header,
    #[token("DATA")]
    Data,
    #[token("ENDSEC")]
    Endsec,
    #[token("ISO-10303-21")]
    StartTag,
    #[token("END-ISO-10303-21")]
    EndTag,
    #[regex(r"\'[^']*\'", |lex| lex.slice().trim_start_matches('\'').trim_end_matches('\'').to_owned())]
    String(String),
    #[regex(r"[^\s;='/]+", |lex| lex.slice().to_owned())]
    Definition(String),
}

impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Comments => write!(f, "/**/"),
            Token::Whitespace => write!(f, " "),
            Token::Eq => write!(f, "="),
            Token::Sem => write!(f, ";"),
            Token::Reference(s) => write!(f, "#{}", s),
            Token::Header => write!(f, "HEADER"),
            Token::Data => write!(f, "DATA"),
            Token::Endsec => write!(f, "ENDSEC"),
            Token::StartTag => write!(f, "ISO-10303-21"),
            Token::EndTag => write!(f, "END-ISO-10303-21"),
            Token::Definition(s) => write!(f, "{}", s),
            Token::String(s) => write!(f, "'{}'", s),
        }
    }
}

struct BufferedReader<R: Read> {
    reader: R,
    buffer: Buffer,
}

impl<R: Read> BufferedReader<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            buffer: Buffer::with_capacity(BUFFER_SIZE_START),
        }
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

    /// Updates the buffer capacity by the growth factor.
    pub fn update_buffer(&mut self) {
        let new_capacity = BUFFER_GROWTH_FACTOR * self.buffer.capacity();
        self.buffer.grow(new_capacity);
    }

    /// Fills the buffer with data from the reader.
    pub fn fill_buffer(&mut self) -> Result<()> {
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

pub struct TokenIterator<R: Read> {
    buffer: BufferedReader<R>,
}

impl<R: Read> TokenIterator<R> {
    pub fn new(reader: R) -> Self {
        Self {
            buffer: BufferedReader::new(reader),
        }
    }
}

impl<R: Read> Iterator for TokenIterator<R> {
    type Item = Result<Token>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let s = match self.buffer.as_str() {
                Ok(s) => s,
                Err(e) => return Some(Err(e)),
            };
            let lexer: logos::Lexer<Token> = Token::lexer(s);
            let mut lexer = lexer.spanned();

            match lexer.next() {
                Some((Ok(token), span)) => {
                    self.buffer.consumed(span.end);
                    return Some(Ok(token));
                }
                Some((Err(_), _)) => {
                    // we assume that we need to update the buffer
                    self.buffer.update_buffer();
                }
                None => {}
            }

            match self.buffer.fill_buffer() {
                Ok(_) => {}
                Err(Error::EndOfInput()) => {
                    return None;
                }
                Err(e) => return Some(Err(e)),
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn test_tokens_iterator_comments() {
        let mut tokens = TokenIterator::new(Cursor::new("/* HELLO WORLD */ HEADER"));

        assert_eq!(Token::Header, tokens.next().unwrap().unwrap());
        assert!(tokens.next().is_none());

        let mut tokens = TokenIterator::new(Cursor::new(
            "/* HELLO WORLD
        Some other line
        */

        HEADER",
        ));

        assert_eq!(Token::Header, tokens.next().unwrap().unwrap());
        assert!(tokens.next().is_none());
    }

    #[test]
    fn test_tokens_iterator_identifier() {
        let mut tokens = TokenIterator::new(Cursor::new("HEADER SOME_IDENTIFIER"));

        assert_eq!(Token::Header, tokens.next().unwrap().unwrap());
        assert_eq!(
            Token::Definition("SOME_IDENTIFIER".to_string()),
            tokens.next().unwrap().unwrap()
        );
        assert!(tokens.next().is_none());
    }

    #[test]
    fn test_string() {
        let mut tokens = TokenIterator::new(Cursor::new("'Hello World'"));

        assert_eq!(
            Token::String("Hello World".to_string()),
            tokens.next().unwrap().unwrap()
        );
        assert!(tokens.next().is_none());
    }

    #[test]
    fn test_reference() {
        let mut tokens = TokenIterator::new(Cursor::new("#1 # 2"));

        assert_eq!(Token::Reference(1u64), tokens.next().unwrap().unwrap());
        assert_eq!(Token::Reference(2u64), tokens.next().unwrap().unwrap());
        assert!(tokens.next().is_none());
    }

    #[test]
    fn test_sym() {
        let mut tokens = TokenIterator::new(Cursor::new("=;(),#&$.*"));

        assert_eq!(Token::Eq, tokens.next().unwrap().unwrap());
        assert_eq!(Token::Sem, tokens.next().unwrap().unwrap());
        assert_eq!(
            Token::Definition("(),#&$.*".to_string()),
            tokens.next().unwrap().unwrap()
        );
        assert!(tokens.next().is_none());
    }

    #[test]
    fn test_tokens_iterator_cube() {
        let reader = Cursor::new(include_bytes!("../../../../../test_data/cube.stp"));
        let tokens = TokenIterator::new(reader);

        for token in tokens {
            let token = token.unwrap();
            match token {
                Token::Sem => println!(";"),
                _ => print!("{}", token),
            }
        }

        println!();
    }
}
