use std::iter::Peekable;

use crate::{Error, Result};

/// A tokenizer that reads characters and returns simple tokens.
pub struct Tokenizer<P: Iterator<Item = Result<char>>> {
    /// The underlying parser that reads characters.
    parser: Peekable<P>,

    /// Indicates if we are currently inside a string.
    is_inside_string: bool,
}

/// The different tokens that are returned by the tokenizer.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Token {
    Whitespace,
    Comment,
    Character(char),
}

impl<P: Iterator<Item = Result<char>>> Tokenizer<P> {
    /// Creates a new tokenizer from the given character parser.
    ///
    /// # Arguments
    /// * `parser` - The character parser to read from.
    pub fn new(parser: P) -> Self {
        Tokenizer {
            parser: parser.peekable(),
            is_inside_string: false,
        }
    }

    /// Converts the parser to a string.
    #[allow(dead_code)]
    pub fn into_string(self) -> Result<String> {
        let mut result = String::new();

        for token in self {
            match token {
                Ok(Token::Whitespace) => result.push('\n'),
                Ok(Token::Comment) => result.push_str("/**/"),
                Ok(Token::Character(ch)) => result.push(ch),
                Err(err) => return Err(err),
            }
        }

        Ok(result)
    }

    /// Skips until the predicate is not true anymore.
    /// Returns an error if the end of the input is reached.
    ///
    /// # Arguments
    /// * `predicate` - The predicate to check for.
    /// * `fail_on_eof` - Whether to fail if the end of the input is reached.
    fn skip_until(&mut self, predicate: impl Fn(char) -> bool, fail_on_eof: bool) -> Result<()> {
        loop {
            match self.parser.peek() {
                Some(ch) => match ch {
                    Ok(ch) => {
                        if !predicate(*ch) {
                            return Ok(());
                        }
                    }
                    Err(err) => return Err(err.clone()),
                },
                None => {
                    if fail_on_eof {
                        return Err(Error::EndOfInput());
                    } else {
                        return Ok(());
                    }
                }
            }

            if let Some(Err(err)) = self.parser.next() {
                return Err(err);
            }
        }
    }
}

impl<P: Iterator<Item = Result<char>>> Iterator for Tokenizer<P> {
    type Item = Result<Token>;

    fn next(&mut self) -> Option<Result<Token>> {
        let ch = match self.parser.next() {
            Some(ch) => match ch {
                Ok(ch) => ch,
                Err(err) => return Some(Err(err)),
            },
            None => return None,
        };

        // case 1: check for string start or end
        if ch == '\'' {
            self.is_inside_string = !self.is_inside_string;
            return Some(Ok(Token::Character('\'')));
        }

        // case 2: check if we are inside a string
        if self.is_inside_string {
            return Some(Ok(Token::Character(ch)));
        }

        // case 3: we encounter whitespace and are outside a string
        if ch.is_whitespace() {
            if let Err(err) = self.skip_until(|ch| ch.is_whitespace(), false) {
                return Some(Err(err));
            }
            return Some(Ok(Token::Whitespace));
        }

        // case 4: we may have encountered a comment and are outside a string
        if ch == '/' {
            if let Some(Ok(next_char)) = self.parser.peek() {
                if *next_char != '*' {
                    return Some(Ok(Token::Character('/')));
                }

                loop {
                    if let Err(err) = self.skip_until(|ch| ch != '*', true) {
                        return Some(Err(err));
                    }

                    self.parser.next();

                    if let Some(Ok(next_char)) = self.parser.peek() {
                        if *next_char == '/' {
                            self.parser.next();
                            return Some(Ok(Token::Comment));
                        }
                    }
                }
            } else {
                return Some(Ok(Token::Character('/')));
            }
        }

        // case 5: we are outside a string and have a normal character
        Some(Ok(Token::Character(ch)))
    }
}

#[cfg(test)]
mod test {
    use std::io::Cursor;

    use crate::step::reader::plain_parser::char_reader::CharReader;

    use super::*;

    #[test]
    fn test_tokenizer_single_word_with_padding() {
        let input = "    Hello\n  ";
        let mut parser = Tokenizer::new(input.chars().map(Ok));

        assert_eq!(parser.next().unwrap().unwrap(), Token::Whitespace);
        assert_eq!(parser.next().unwrap().unwrap(), Token::Character('H'));
        assert_eq!(parser.next().unwrap().unwrap(), Token::Character('e'));
        assert_eq!(parser.next().unwrap().unwrap(), Token::Character('l'));
        assert_eq!(parser.next().unwrap().unwrap(), Token::Character('l'));
        assert_eq!(parser.next().unwrap().unwrap(), Token::Character('o'));
        assert_eq!(parser.next().unwrap().unwrap(), Token::Whitespace);
        assert!(parser.next().is_none());
    }

    #[test]
    fn test_tokenizer_single_word_with_comment() {
        let input = "    Hello\n /*asd*/  ";
        let mut parser = Tokenizer::new(input.chars().map(Ok));

        assert_eq!(parser.next().unwrap().unwrap(), Token::Whitespace);
        assert_eq!(parser.next().unwrap().unwrap(), Token::Character('H'));
        assert_eq!(parser.next().unwrap().unwrap(), Token::Character('e'));
        assert_eq!(parser.next().unwrap().unwrap(), Token::Character('l'));
        assert_eq!(parser.next().unwrap().unwrap(), Token::Character('l'));
        assert_eq!(parser.next().unwrap().unwrap(), Token::Character('o'));
        assert_eq!(parser.next().unwrap().unwrap(), Token::Whitespace);
        assert_eq!(parser.next().unwrap().unwrap(), Token::Comment);
        assert_eq!(parser.next().unwrap().unwrap(), Token::Whitespace);
        assert!(parser.next().is_none());
    }

    #[test]
    fn test_tokenizer_single_word_with_comment_and_string() {
        let input = "    Hello\n /*asd''*/ '/**/' ";
        let mut parser = Tokenizer::new(input.chars().map(Ok));

        assert_eq!(parser.next().unwrap().unwrap(), Token::Whitespace);
        assert_eq!(parser.next().unwrap().unwrap(), Token::Character('H'));
        assert_eq!(parser.next().unwrap().unwrap(), Token::Character('e'));
        assert_eq!(parser.next().unwrap().unwrap(), Token::Character('l'));
        assert_eq!(parser.next().unwrap().unwrap(), Token::Character('l'));
        assert_eq!(parser.next().unwrap().unwrap(), Token::Character('o'));
        assert_eq!(parser.next().unwrap().unwrap(), Token::Whitespace);
        assert_eq!(parser.next().unwrap().unwrap(), Token::Comment);
        assert_eq!(parser.next().unwrap().unwrap(), Token::Whitespace);
        assert_eq!(parser.next().unwrap().unwrap(), Token::Character('\''));
        assert_eq!(parser.next().unwrap().unwrap(), Token::Character('/'));
        assert_eq!(parser.next().unwrap().unwrap(), Token::Character('*'));
        assert_eq!(parser.next().unwrap().unwrap(), Token::Character('*'));
        assert_eq!(parser.next().unwrap().unwrap(), Token::Character('/'));
        assert_eq!(parser.next().unwrap().unwrap(), Token::Character('\''));
        assert_eq!(parser.next().unwrap().unwrap(), Token::Whitespace);
        assert!(parser.next().is_none());
    }

    #[test]
    fn test_tokenizer_complex() {
        let input = r#"
        ISO-10303-21;

HEADER;
	FILE_DESCRIPTION(
		/* description */			(''),
		/* implementation_level */	'2;1');

	FILE_NAME(
		/* name */					'some-file.stp',
		/* date */					'2024-02-09T12:37:21+01:00',
		/* author */				(''),
		/* organization */			(''),
		/* preprocessor_version */	'Random STP   generator',
		/* originating_system */	'Some System',
		/* authorization */			'');

	FILE_SCHEMA(
		/* protocol */				(''));
ENDSEC;
        "#;

        let parser = Tokenizer::new(input.chars().map(Ok));
        let mut cleaned_lines: Vec<String> = Vec::new();

        let mut buffer = String::new();
        for token in parser {
            let token = token.unwrap();

            match token {
                Token::Character(ch) => buffer.push(ch),
                _ => {
                    if !buffer.is_empty() {
                        cleaned_lines.push(buffer.clone());
                        buffer.clear();
                    }
                }
            }
        }

        if !buffer.is_empty() {
            cleaned_lines.push(buffer);
        }

        let resulting_lines = vec![
            "ISO-10303-21;",
            "HEADER;",
            "FILE_DESCRIPTION(",
            "(''),",
            "'2;1');",
            "FILE_NAME(",
            "'some-file.stp',",
            "'2024-02-09T12:37:21+01:00',",
            "(''),",
            "(''),",
            "'Random STP   generator',",
            "'Some System',",
            "'');",
            "FILE_SCHEMA(",
            "(''));",
            "ENDSEC;",
        ];

        assert_eq!(cleaned_lines.len(), resulting_lines.len());

        for (cleaned_line, resulting_line) in cleaned_lines.iter().zip(resulting_lines.iter()) {
            assert_eq!(cleaned_line.trim(), *resulting_line);
        }
    }

    #[test]
    fn test_tokenizer_complex2() {
        let mut input = Cursor::new(include_bytes!("../../../../../test_data/wiki.stp"));
        let char_reader = CharReader::new(&mut input);
        let parser = Tokenizer::new(char_reader);

        let output = include_str!("../../../../../test_data/wiki-normalized.stp");
        assert_eq!(output, parser.into_string().unwrap());
    }
}
