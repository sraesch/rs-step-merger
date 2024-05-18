use std::fmt::Display;

use logos::Logos;

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
    #[token("(")]
    BrO,
    #[token(")")]
    BrC,
    #[token(",")]
    Com,
    #[token("#")]
    Hash,
    #[token("&")]
    Amp,
    #[token("$")]
    Dol,
    #[token(".")]
    Dot,
    #[token("*")]
    Star,
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
    #[regex(r"[A-Za-z][A-Za-z0-9\-_]*", |lex| lex.slice().to_owned())]
    Identifier(String),
    #[regex(r"\'[^']*\'", |lex| lex.slice().trim_start_matches('\'').trim_end_matches('\'').to_owned())]
    String(String),
    #[regex(r"-?(?:0|[1-9]\d*)(?:\.\d*)?(?:[eE][+-]?\d+)?", |lex| lex.slice().to_owned())]
    Number(String),
}

impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Comments => write!(f, "/**/"),
            Token::Whitespace => write!(f, " "),
            Token::Eq => write!(f, "="),
            Token::Sem => write!(f, ";"),
            Token::BrO => write!(f, "("),
            Token::BrC => write!(f, ")"),
            Token::Com => write!(f, ","),
            Token::Hash => write!(f, "#"),
            Token::Amp => write!(f, "&"),
            Token::Dol => write!(f, "$"),
            Token::Dot => write!(f, "."),
            Token::Star => write!(f, "*"),
            Token::Header => write!(f, "HEADER"),
            Token::Data => write!(f, "DATA"),
            Token::Endsec => write!(f, "ENDSEC"),
            Token::StartTag => write!(f, "ISO-10303-21"),
            Token::EndTag => write!(f, "END-ISO-10303-21"),
            Token::Identifier(s) => write!(f, "{}", s),
            Token::String(s) => write!(f, "'{}'", s),
            Token::Number(s) => write!(f, "{}", s),
        }
    }
}

#[cfg(test)]
mod test {
    use std::fs;

    use logos::Logos;

    use super::Token;

    #[test]
    fn test_comments() {
        let mut lex = Token::lexer("/* HELLO WORLD */ HEADER");
        assert_eq!(Some(Ok(Token::Header)), lex.next());
        assert_eq!(None, lex.next());
        let mut lex = Token::lexer(
            "/* HELLO WORLD 
        Some other line
        */ 
        
        HEADER",
        );
        assert_eq!(Some(Ok(Token::Header)), lex.next());
        assert_eq!(None, lex.next());
    }

    #[test]
    fn test_identifier() {
        let mut lex = Token::lexer("HEADER SOME_IDENTIFIER");
        assert_eq!(Some(Ok(Token::Header)), lex.next());
        assert_eq!(
            Some(Ok(Token::Identifier("SOME_IDENTIFIER".to_string()))),
            lex.next()
        );
        assert_eq!(None, lex.next());
    }

    #[test]
    fn test_string() {
        let mut lex = Token::lexer("'Hello World'");
        assert_eq!(
            Some(Ok(Token::String("Hello World".to_string()))),
            lex.next()
        );
        assert_eq!(None, lex.next());
    }

    #[test]
    fn test_number() {
        let mut lex = Token::lexer("42 4.522 5e-43");
        assert_eq!(Some(Ok(Token::Number("42".to_string()))), lex.next());
        assert_eq!(Some(Ok(Token::Number("4.522".to_string()))), lex.next());
        assert_eq!(Some(Ok(Token::Number("5e-43".to_string()))), lex.next());
        assert_eq!(None, lex.next());
    }

    #[test]
    fn test_sym() {
        let mut lex = Token::lexer("=;(),#&$.*");
        assert_eq!(Some(Ok(Token::Eq)), lex.next());
        assert_eq!(Some(Ok(Token::Sem)), lex.next());
        assert_eq!(Some(Ok(Token::BrO)), lex.next());
        assert_eq!(Some(Ok(Token::BrC)), lex.next());
        assert_eq!(Some(Ok(Token::Com)), lex.next());
        assert_eq!(Some(Ok(Token::Hash)), lex.next());
        assert_eq!(Some(Ok(Token::Amp)), lex.next());
        assert_eq!(Some(Ok(Token::Dol)), lex.next());
        assert_eq!(Some(Ok(Token::Dot)), lex.next());
        assert_eq!(Some(Ok(Token::Star)), lex.next());
        assert_eq!(None, lex.next());
    }

    #[test]
    fn test_error() {
        let mut lex = Token::lexer("+ HEADER");
        assert_eq!(Some(Err(())), lex.next());
        assert_eq!(Some(Ok(Token::Header)), lex.next());
        assert_eq!(None, lex.next());
    }

    #[test]
    fn test_file_wiki() {
        let filename = "../test_data/wiki.stp";
        let s = fs::read_to_string(filename);
        match s {
            Ok(s) => {
                let lex: Vec<_> = Token::lexer(&s).collect();
                assert!(!lex.iter().any(|l| l.is_err()))
            }
            Err(e) => panic!("Failed to read {filename}: {e:?}"),
        }
    }

    #[test]
    fn test_file_1() {
        let filename = "../test_data/1.stp";
        let s = fs::read_to_string(filename);
        match s {
            Ok(s) => {
                let lex: Vec<_> = Token::lexer(&s).collect();
                assert!(!lex.iter().any(|l| l.is_err()))
            }
            Err(e) => panic!("Failed to read {filename}: {e:?}"),
        }
    }

    #[test]
    fn test_file_2() {
        let filename = "../test_data/2.stp";
        let s = fs::read_to_string(filename);
        match s {
            Ok(s) => {
                let lex: Vec<_> = Token::lexer(&s).collect();
                assert!(!lex.iter().any(|l| l.is_err()))
            }
            Err(e) => panic!("Failed to read {filename}: {e:?}"),
        }
    }

    #[test]
    fn test_file_cube() {
        let filename = "../test_data/cube.stp";
        let s = fs::read_to_string(filename);
        match s {
            Ok(s) => {
                let lex: Vec<_> = Token::lexer(&s).collect();
                assert!(!lex.iter().any(|l| l.is_err()));

                for l in lex {
                    println!("{:?}", l);
                }
            }
            Err(e) => panic!("Failed to read {filename}: {e:?}"),
        }
    }
}
