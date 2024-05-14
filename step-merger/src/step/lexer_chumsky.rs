use std::{
    fmt::{self, Debug, Display},
    ops::Range,
};

use chumsky::{extra::ParserExtra, prelude::*};

pub(crate) type Span = SimpleSpan<usize>;

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Spanned<T> {
    pub v: T,
    pub s: Span,
}

impl<T> Spanned<T> {
    fn new(v: T, s: Span) -> Self {
        Self { v, s }
    }
}

impl<T: PartialEq> Spanned<T> {
    pub fn cmp_unspanned(&self, other: &Self) -> bool {
        self.v == other.v
    }
}

impl<T> From<(T, Span)> for Spanned<T> {
    fn from((v, s): (T, Span)) -> Self {
        Self::new(v, s)
    }
}

impl<T> From<(T, Range<usize>)> for Spanned<T> {
    fn from((v, s): (T, Range<usize>)) -> Self {
        Self::new(v, s.into())
    }
}

impl<T> From<(T, Range<usize>)> for Box<Spanned<T>> {
    fn from(v: (T, Range<usize>)) -> Self {
        Box::new(v.into())
    }
}

impl<T: Debug> Debug for Spanned<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({:?}, {}..{}).into()", self.v, self.s.start, self.s.end)
    }
}

impl<T: Display> Display for Spanned<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.v)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Symbol {
    Eq,
    Sem,
    BrO,
    BrC,
    Com,
    Hash,
    Amp,
    Dol,
    Dot,
    Star,
}

impl<'src> Symbol {
    fn parse_symbol<E: ParserExtra<'src, &'src str>>(
    ) -> impl Parser<'src, &'src str, Symbol, E> + Copy {
        choice((
            just('=').map(|_| Symbol::Eq),
            just(';').map(|_| Symbol::Sem),
            just('(').map(|_| Symbol::BrO),
            just(')').map(|_| Symbol::BrC),
            just(',').map(|_| Symbol::Com),
            just('#').map(|_| Symbol::Hash),
            just('&').map(|_| Symbol::Amp),
            just('$').map(|_| Symbol::Dol),
            just('.').map(|_| Symbol::Dot),
            just('*').map(|_| Symbol::Star),
        ))
    }
}

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Symbol::Eq => write!(f, "="),
            Symbol::Sem => write!(f, ";"),
            Symbol::BrO => write!(f, "("),
            Symbol::BrC => write!(f, ")"),
            Symbol::Com => write!(f, ","),
            Symbol::Hash => write!(f, "#"),
            Symbol::Amp => write!(f, "&"),
            Symbol::Dol => write!(f, "$"),
            Symbol::Dot => write!(f, "."),
            Symbol::Star => write!(f, "*"),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Token<'src> {
    Header,
    Data,
    Endsec,
    String(&'src str),
    Identifier(&'src str),
    Integer(isize),
    Float(&'src str),
    Reference(usize),
    Sym(Symbol),
    Comment(&'src str),
    Enum(&'src str),
    StartTag,
    EndTag,
}

impl fmt::Display for Token<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Token::Header => write!(f, "HEADER"),
            Token::Data => write!(f, "DATA"),
            Token::Endsec => write!(f, "ENDSEC"),
            Token::String(s) => write!(f, "'{s}'"),
            Token::Identifier(i) => write!(f, "{i}"),
            Token::Integer(n) => write!(f, "{n}"),
            Token::Float(n) => write!(f, "{n}"),
            Token::Reference(r) => write!(f, "#{r}"),
            Token::Sym(s) => write!(f, "{s}"),
            Token::Comment(c) => write!(f, "/*{c}*/"),
            Token::StartTag => write!(f, "ISO-10303-21"),
            Token::EndTag => write!(f, "END-ISO-10303-21"),
            Token::Enum(s) => write!(f, ".{s}."),
        }
    }
}

impl<'src> Token<'src> {
    fn parser<E: ParserExtra<'src, &'src str>>(
    ) -> impl Parser<'src, &'src str, Token<'src>, E> + Copy {
        choice((
            Self::parse_enum(),
            Self::parse_float(),
            Self::parse_integer_token(),
            Self::parse_reference(),
            Self::step_identifier(),
            Symbol::parse_symbol().map(Self::Sym),
            Self::parse_string(),
            Self::parse_comment(),
        ))
    }

    fn parse_float<E: ParserExtra<'src, &'src str>>(
    ) -> impl Parser<'src, &'src str, Token<'src>, E> + Copy {
        Self::parse_signed_integer()
            .then(just('.'))
            .then(Self::parse_integer().or_not())
            .then(
                just('E')
                    .or(just('e'))
                    .then(just('+').or_not().then(Self::parse_signed_integer())) // This allows E+-42
                    .or_not(),
            )
            .to_slice()
            .map(Self::Float)
    }

    #[must_use]
    fn parse_integer<E: ParserExtra<'src, &'src str>>(
    ) -> impl Parser<'src, &'src str, &'src str, E> + Copy {
        any()
            .filter(|n: &char| n.is_ascii_digit())
            .repeated()
            .at_least(1)
            .to_slice()
    }

    #[must_use]
    fn parse_signed_integer<E: ParserExtra<'src, &'src str>>(
    ) -> impl Parser<'src, &'src str, &'src str, E> + Copy {
        just("-").or_not().then(Self::parse_integer()).to_slice()
    }

    fn parse_integer_token<E: ParserExtra<'src, &'src str>>(
    ) -> impl Parser<'src, &'src str, Token<'src>, E> + Copy {
        Self::parse_signed_integer().map(|n: &str| Self::Integer(n.parse::<isize>().unwrap()))
    }

    fn parse_reference<E: ParserExtra<'src, &'src str>>(
    ) -> impl Parser<'src, &'src str, Token<'src>, E> + Copy {
        just("#")
            .ignore_then(Self::parse_integer())
            .map(|s: &str| Self::Reference(s.parse::<usize>().unwrap()))
    }

    fn parse_enum<E: ParserExtra<'src, &'src str>>(
    ) -> impl Parser<'src, &'src str, Token<'src>, E> + Copy {
        just(".")
            .ignore_then(Self::step_identifier().to_slice())
            .then_ignore(just("."))
            .map(|s: &str| Self::Enum(s))
    }

    fn step_identifier<E: ParserExtra<'src, &'src str>>(
    ) -> impl Parser<'src, &'src str, Token<'src>, E> + Copy {
        let step_identifier = any()
            .filter(|c: &char| c.is_ascii_uppercase() || c.is_ascii_digit())
            .then(
                any()
                    .filter(|c: &char| {
                        c.is_ascii_uppercase() || *c == '-' || *c == '_' || c.is_ascii_digit()
                    })
                    .repeated(),
            )
            .to_slice();

        step_identifier.map(|ident| match ident {
            "ISO-10303-21" => Self::StartTag,
            "END-ISO-10303-21" => Self::EndTag,
            "ENDSEC" => Self::Endsec,
            "DATA" => Self::Data,
            "HEADER" => Self::Header,
            _ => Self::Identifier(ident),
        })
    }

    // Comments are ignored in lexer right now. Can be enabled if necessary to get comment content.
    fn parse_comment<E: ParserExtra<'src, &'src str>>(
    ) -> impl Parser<'src, &'src str, Token<'src>, E> + Copy {
        just("/*")
            .ignore_then(any().and_is(just("*/").not()).repeated().to_slice())
            .then_ignore(just("*/"))
            .map(Self::Comment)
    }

    fn parse_string<E: ParserExtra<'src, &'src str>>(
    ) -> impl Parser<'src, &'src str, Token<'src>, E> + Copy {
        just("'")
            .ignore_then(any().and_is(just("'").not()).repeated().to_slice())
            .then_ignore(just("'"))
            .map(Self::String)
    }

    pub fn lexer_iter(
    ) -> impl IterParser<'src, &'src str, Spanned<Token<'src>>, extra::Err<EmptyErr>> {
        Self::parser()
            .map_with(|tok, e| (tok, e.span()).into())
            .padded()
            .recover_with(skip_then_retry_until(any().ignored(), end()))
            .repeated()
    }

    pub fn lexer() -> impl Parser<'src, &'src str, Vec<Spanned<Token<'src>>>, extra::Err<EmptyErr>>
    {
        Self::lexer_iter().collect()
    }
}

#[cfg(test)]
mod test {
    use std::fs;

    use super::*;

    #[test]
    fn test_sym() {
        run_test("&", vec![(Token::Sym(Symbol::Amp), 0..1).into()]);
        run_test(
            "& /* Hello World! */ &",
            vec![
                (Token::Sym(Symbol::Amp), 0..1).into(),
                (Token::Comment(" Hello World! "), 2..20).into(),
                (Token::Sym(Symbol::Amp), 21..22).into(),
            ],
        );
    }

    #[test]
    fn test_enum() {
        run_test(".TEST.", vec![(Token::Enum("TEST"), 0..6).into()]);
        run_test(".TRUE.", vec![(Token::Enum("TRUE"), 0..6).into()]);
    }

    #[test]
    fn test_identifier() {
        run_test(
            "ISO-42424242",
            vec![(Token::Identifier("ISO-42424242"), 0..12).into()],
        );

        run_test("END-ISO-10303-21", vec![(Token::EndTag, 0..16).into()]);

        run_test("HEADER", vec![(Token::Header, 0..6).into()]);
    }

    #[test]
    fn test_numbers() {
        run_test("1234", vec![(Token::Integer(1234), 0..4).into()]);
        run_test("-1234", vec![(Token::Integer(-1234), 0..5).into()]);
        run_test("-1234.42", vec![(Token::Float("-1234.42"), 0..8).into()]);
        run_test(
            "-1234.42e44",
            vec![(Token::Float("-1234.42e44"), 0..11).into()],
        );
    }

    #[test]
    fn test_references() {
        run_test("#42", vec![(Token::Reference(42), 0..3).into()]);
        run_test(
            "#42 ; #44=HELLO_WORLD",
            vec![
                (Token::Reference(42), 0..3).into(),
                (Token::Sym(Symbol::Sem), 4..5).into(),
                (Token::Reference(44), 6..9).into(),
                (Token::Sym(Symbol::Eq), 9..10).into(),
                (Token::Identifier("HELLO_WORLD"), 10..21).into(),
            ],
        );
    }

    #[test]
    fn wiki_example() {
        let filename = "../test_data/wiki.stp";
        let s = fs::read_to_string(filename);

        use super::Symbol::*;
        use super::Token::*;

        match s {
            Ok(s) => run_test(
                &s,
                vec![
                    (StartTag, 0..12).into(),
                    (Sym(Sem), 12..13).into(),
                    (Header, 14..20).into(),
                    (Sym(Sem), 20..21).into(),
                    (Identifier("FILE_DESCRIPTION"), 22..38).into(),
                    (Sym(BrO), 38..39).into(),
                    (Comment(" description "), 40..57).into(),
                    (Sym(BrO), 58..59).into(),
                    (
                        String("A minimal AP214 example with a single part"),
                        59..103,
                    )
                        .into(),
                    (Sym(BrC), 103..104).into(),
                    (Sym(Com), 104..105).into(),
                    (Comment(" implementation_level "), 106..132).into(),
                    (String("2;1"), 133..138).into(),
                    (Sym(BrC), 138..139).into(),
                    (Sym(Sem), 139..140).into(),
                    (Identifier("FILE_NAME"), 141..150).into(),
                    (Sym(BrO), 150..151).into(),
                    (Comment(" name "), 152..162).into(),
                    (String("demo"), 163..169).into(),
                    (Sym(Com), 169..170).into(),
                    (Comment(" time_stamp "), 171..187).into(),
                    (String("2003-12-27T11:57:53"), 188..209).into(),
                    (Sym(Com), 209..210).into(),
                    (Comment(" author "), 211..223).into(),
                    (Sym(BrO), 224..225).into(),
                    (String("Lothar Klein"), 225..239).into(),
                    (Sym(BrC), 239..240).into(),
                    (Sym(Com), 240..241).into(),
                    (Comment(" organization "), 242..260).into(),
                    (Sym(BrO), 261..262).into(),
                    (String("LKSoft"), 262..270).into(),
                    (Sym(BrC), 270..271).into(),
                    (Sym(Com), 271..272).into(),
                    (Comment(" preprocessor_version "), 273..299).into(),
                    (String(" "), 300..303).into(),
                    (Sym(Com), 303..304).into(),
                    (Comment(" originating_system "), 305..329).into(),
                    (String("IDA-STEP"), 330..340).into(),
                    (Sym(Com), 340..341).into(),
                    (Comment(" authorization "), 342..361).into(),
                    (String(" "), 362..365).into(),
                    (Sym(BrC), 365..366).into(),
                    (Sym(Sem), 366..367).into(),
                    (Identifier("FILE_SCHEMA"), 368..379).into(),
                    (Sym(BrO), 380..381).into(),
                    (Sym(BrO), 381..382).into(),
                    (String("AUTOMOTIVE_DESIGN { 1 0 10303 214 2 1 1}"), 382..424).into(),
                    (Sym(BrC), 424..425).into(),
                    (Sym(BrC), 425..426).into(),
                    (Sym(Sem), 426..427).into(),
                    (Endsec, 428..434).into(),
                    (Sym(Sem), 434..435).into(),
                    (Data, 436..440).into(),
                    (Sym(Sem), 440..441).into(),
                    (Reference(10), 442..445).into(),
                    (Sym(Eq), 445..446).into(),
                    (Identifier("ORGANIZATION"), 446..458).into(),
                    (Sym(BrO), 458..459).into(),
                    (String("O0001"), 459..466).into(),
                    (Sym(Com), 466..467).into(),
                    (String("LKSoft"), 467..475).into(),
                    (Sym(Com), 475..476).into(),
                    (String("company"), 476..485).into(),
                    (Sym(BrC), 485..486).into(),
                    (Sym(Sem), 486..487).into(),
                    (Reference(11), 488..491).into(),
                    (Sym(Eq), 491..492).into(),
                    (Identifier("PRODUCT_DEFINITION_CONTEXT"), 492..518).into(),
                    (Sym(BrO), 518..519).into(),
                    (String("part definition"), 519..536).into(),
                    (Sym(Com), 536..537).into(),
                    (Reference(12), 537..540).into(),
                    (Sym(Com), 540..541).into(),
                    (String("manufacturing"), 541..556).into(),
                    (Sym(BrC), 556..557).into(),
                    (Sym(Sem), 557..558).into(),
                    (Reference(12), 559..562).into(),
                    (Sym(Eq), 562..563).into(),
                    (Identifier("APPLICATION_CONTEXT"), 563..582).into(),
                    (Sym(BrO), 582..583).into(),
                    (String("mechanical design"), 583..602).into(),
                    (Sym(BrC), 602..603).into(),
                    (Sym(Sem), 603..604).into(),
                    (Reference(13), 605..608).into(),
                    (Sym(Eq), 608..609).into(),
                    (Identifier("APPLICATION_PROTOCOL_DEFINITION"), 609..640).into(),
                    (Sym(BrO), 640..641).into(),
                    (String(""), 641..643).into(),
                    (Sym(Com), 643..644).into(),
                    (String("automotive_design"), 644..663).into(),
                    (Sym(Com), 663..664).into(),
                    (Integer(2003), 664..668).into(),
                    (Sym(Com), 668..669).into(),
                    (Reference(12), 669..672).into(),
                    (Sym(BrC), 672..673).into(),
                    (Sym(Sem), 673..674).into(),
                    (Reference(14), 675..678).into(),
                    (Sym(Eq), 678..679).into(),
                    (Identifier("PRODUCT_DEFINITION"), 679..697).into(),
                    (Sym(BrO), 697..698).into(),
                    (String("0"), 698..701).into(),
                    (Sym(Com), 701..702).into(),
                    (Sym(Dol), 702..703).into(),
                    (Sym(Com), 703..704).into(),
                    (Reference(15), 704..707).into(),
                    (Sym(Com), 707..708).into(),
                    (Reference(11), 708..711).into(),
                    (Sym(BrC), 711..712).into(),
                    (Sym(Sem), 712..713).into(),
                    (Reference(15), 714..717).into(),
                    (Sym(Eq), 717..718).into(),
                    (Identifier("PRODUCT_DEFINITION_FORMATION"), 718..746).into(),
                    (Sym(BrO), 746..747).into(),
                    (String("1"), 747..750).into(),
                    (Sym(Com), 750..751).into(),
                    (Sym(Dol), 751..752).into(),
                    (Sym(Com), 752..753).into(),
                    (Reference(16), 753..756).into(),
                    (Sym(BrC), 756..757).into(),
                    (Sym(Sem), 757..758).into(),
                    (Reference(16), 759..762).into(),
                    (Sym(Eq), 762..763).into(),
                    (Identifier("PRODUCT"), 763..770).into(),
                    (Sym(BrO), 770..771).into(),
                    (String("A0001"), 771..778).into(),
                    (Sym(Com), 778..779).into(),
                    (String("Test Part 1"), 779..792).into(),
                    (Sym(Com), 792..793).into(),
                    (String(""), 793..795).into(),
                    (Sym(Com), 795..796).into(),
                    (Sym(BrO), 796..797).into(),
                    (Reference(18), 797..800).into(),
                    (Sym(BrC), 800..801).into(),
                    (Sym(BrC), 801..802).into(),
                    (Sym(Sem), 802..803).into(),
                    (Reference(17), 804..807).into(),
                    (Sym(Eq), 807..808).into(),
                    (Identifier("PRODUCT_RELATED_PRODUCT_CATEGORY"), 808..840).into(),
                    (Sym(BrO), 840..841).into(),
                    (String("part"), 841..847).into(),
                    (Sym(Com), 847..848).into(),
                    (Sym(Dol), 848..849).into(),
                    (Sym(Com), 849..850).into(),
                    (Sym(BrO), 850..851).into(),
                    (Reference(16), 851..854).into(),
                    (Sym(BrC), 854..855).into(),
                    (Sym(BrC), 855..856).into(),
                    (Sym(Sem), 856..857).into(),
                    (Reference(18), 858..861).into(),
                    (Sym(Eq), 861..862).into(),
                    (Identifier("PRODUCT_CONTEXT"), 862..877).into(),
                    (Sym(BrO), 877..878).into(),
                    (String(""), 878..880).into(),
                    (Sym(Com), 880..881).into(),
                    (Reference(12), 881..884).into(),
                    (Sym(Com), 884..885).into(),
                    (String(""), 885..887).into(),
                    (Sym(BrC), 887..888).into(),
                    (Sym(Sem), 888..889).into(),
                    (Reference(19), 890..893).into(),
                    (Sym(Eq), 893..894).into(),
                    (Identifier("APPLIED_ORGANIZATION_ASSIGNMENT"), 894..925).into(),
                    (Sym(BrO), 925..926).into(),
                    (Reference(10), 926..929).into(),
                    (Sym(Com), 929..930).into(),
                    (Reference(20), 930..933).into(),
                    (Sym(Com), 933..934).into(),
                    (Sym(BrO), 934..935).into(),
                    (Reference(16), 935..938).into(),
                    (Sym(BrC), 938..939).into(),
                    (Sym(BrC), 939..940).into(),
                    (Sym(Sem), 940..941).into(),
                    (Reference(20), 942..945).into(),
                    (Sym(Eq), 945..946).into(),
                    (Identifier("ORGANIZATION_ROLE"), 946..963).into(),
                    (Sym(BrO), 963..964).into(),
                    (String("id owner"), 964..974).into(),
                    (Sym(BrC), 974..975).into(),
                    (Sym(Sem), 975..976).into(),
                    (Endsec, 977..983).into(),
                    (Sym(Sem), 983..984).into(),
                    (EndTag, 985..1001).into(),
                    (Sym(Sem), 1001..1002).into(),
                ],
            ),
            Err(e) => panic!("Failed to read {filename}: {e:?}"),
        }
    }

    #[test]
    fn test_file_1() {
        let filename = "../test_data/1.stp";
        let s = fs::read_to_string(filename);
        match s {
            Ok(s) => run_large_test(&s),
            Err(e) => panic!("Failed to read {filename}: {e:?}"),
        }
    }

    #[test]
    fn test_file_2() {
        let filename = "../test_data/2.stp";
        let s = fs::read_to_string(filename);
        match s {
            Ok(s) => run_large_test(&s),
            Err(e) => panic!("Failed to read {filename}: {e:?}"),
        }
    }

    fn run_test(src: &str, cmp: Vec<Spanned<Token>>) {
        let (tokens, errs) = Token::lexer().parse(src).into_output_errors();

        println!("{:?}", tokens);
        println!("{:?}", errs);

        assert!(errs.is_empty());
        assert_eq!(tokens, Some(cmp));
    }

    fn run_large_test(src: &str) {
        let (tokens, errs) = Token::lexer().parse(src).into_output_errors();
        let tokens = tokens.expect("No tokens generated");
        let tokens_len = tokens.len();
        let parsed_len = std::mem::size_of::<super::Token>() * tokens_len;
        let src_len = src.len();

        let fac = parsed_len as f64 / src_len as f64;

        println!(
            "Tokens: {} Token Size: {}B Symbol Size: {}B",
            tokens_len,
            std::mem::size_of::<super::Token>(),
            std::mem::size_of::<super::Symbol>()
        );
        println!(
            "Source size: {}B Parsed size: {}B ({:.2}x)",
            src_len, parsed_len, fac
        );

        if !errs.is_empty() {
            panic!("Errors while parsing: {errs:?}");
        }
    }
}
