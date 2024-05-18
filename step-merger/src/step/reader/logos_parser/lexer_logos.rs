use logos::Logos;

#[derive(Logos, Debug, PartialEq)]
pub enum Token<'src> {
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
    #[regex(r"[A-Za-z][A-Za-z0-9\-_]*")]
    Identifier(&'src str),
    #[regex(r"\'[^']*\'", |lex| lex.slice().trim_start_matches('\'').trim_end_matches('\''))]
    String(&'src str),
    #[regex(r"-?(?:0|[1-9]\d*)(?:\.\d*)?(?:[eE][+-]?\d+)?")]
    Number(&'src str),
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
        assert_eq!(Some(Ok(Token::Identifier("SOME_IDENTIFIER"))), lex.next());
        assert_eq!(None, lex.next());
    }

    #[test]
    fn test_string() {
        let mut lex = Token::lexer("'Hello World'");
        assert_eq!(Some(Ok(Token::String("Hello World"))), lex.next());
        assert_eq!(None, lex.next());
    }

    #[test]
    fn test_number() {
        let mut lex = Token::lexer("42 4.522 5e-43");
        assert_eq!(Some(Ok(Token::Number("42"))), lex.next());
        assert_eq!(Some(Ok(Token::Number("4.522"))), lex.next());
        assert_eq!(Some(Ok(Token::Number("5e-43"))), lex.next());
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
}
