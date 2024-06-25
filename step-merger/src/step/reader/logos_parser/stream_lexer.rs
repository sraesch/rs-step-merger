use std::{fmt::Display, iter::Peekable};

use logos::{Logos, SpannedIter};

use crate::{Error, Result};

#[derive(Logos, Debug, Clone, PartialEq)]
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

/// Iterator over the tokens which define the STEP file.
pub struct TokenIterator<'a> {
    /// The Logos lexer peekable iterator.
    it: Peekable<SpannedIter<'a, Token>>,

    /// The number of bytes consumed by the iterator.
    consumed_bytes: usize,
}

impl<'a> TokenIterator<'a> {
    /// Creates a new token iterator from the given string.
    ///
    /// # Arguments
    /// * `src` - The string source to parse.
    pub fn new(src: &'a str) -> Self {
        let lexer = Token::lexer(src);
        let it = lexer.spanned();
        let peekable_it = it.peekable();

        Self {
            it: peekable_it,
            consumed_bytes: 0,
        }
    }

    /// Returns a reference to the internal consumed bytes counter.
    pub fn consumed_bytes(&self) -> usize {
        self.consumed_bytes
    }

    /// Peeks the next token.
    /// Returns `None` if the iterator is at the end.
    pub fn peek(&mut self) -> Option<Result<Token>> {
        match self.it.peek().cloned() {
            Some((Ok(token), _)) => Some(Ok(token)),
            Some((Err(_), _)) => Some(Err(Error::ParsingTokenError())),
            None => None,
        }
    }
}

impl<'a> Iterator for TokenIterator<'a> {
    type Item = Result<Token>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.it.next() {
            Some((Ok(token), span)) => {
                self.consumed_bytes = span.end;
                Some(Ok(token))
            }
            Some((Err(_), _)) => Some(Err(Error::ParsingTokenError())),
            None => None,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_tokens_iterator_comments() {
        let mut tokens = TokenIterator::new("/* HELLO WORLD */ HEADER");

        assert_eq!(Token::Header, tokens.next().unwrap().unwrap());
        assert!(tokens.next().is_none());

        let mut tokens = TokenIterator::new(
            "/* HELLO WORLD
        Some other line
        */

        HEADER",
        );

        assert_eq!(Token::Header, tokens.next().unwrap().unwrap());
        assert!(tokens.next().is_none());
    }

    #[test]
    fn test_tokens_iterator_identifier() {
        let mut tokens = TokenIterator::new("HEADER SOME_IDENTIFIER");

        assert_eq!(Token::Header, tokens.next().unwrap().unwrap());
        assert_eq!(
            Token::Definition("SOME_IDENTIFIER".to_string()),
            tokens.next().unwrap().unwrap()
        );
        assert!(tokens.next().is_none());
    }

    #[test]
    fn test_string() {
        let mut tokens = TokenIterator::new("'Hello World'");

        assert_eq!(
            Token::String("Hello World".to_string()),
            tokens.next().unwrap().unwrap()
        );
        assert!(tokens.next().is_none());
    }

    #[test]
    fn test_reference() {
        let mut tokens = TokenIterator::new("#1 # 2");

        assert_eq!(Token::Reference(1u64), tokens.next().unwrap().unwrap());
        assert_eq!(Token::Reference(2u64), tokens.next().unwrap().unwrap());
        assert!(tokens.next().is_none());
    }

    #[test]
    fn test_sym() {
        let mut tokens = TokenIterator::new("=;(),#&$.*");

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
        let reader = include_str!("../../../../../test_data/cube.stp");
        let tokens = TokenIterator::new(reader);

        for token in tokens {
            let token = token.unwrap();
            match token {
                Token::Sem => println!(";"),
                _ => print!("{}", token),
            }
        }
    }
}
