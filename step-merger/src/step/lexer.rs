use std::{
    fmt::{self, Debug, Display},
    ops::Range,
};

use chumsky::{extra::ParserExtra, prelude::*};

pub(crate) type Span = SimpleSpan<usize>;

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Spanned<T> {
    v: T,
    s: Span,
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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Symbol {
    Eq,
    Sem,
    BrO,
    BrC,
    Com,
    Hash,
    Amp,
    Dol,
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
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Token<'src> {
    Header,
    Data,
    Endsec,
    String(&'src str),
    Identifier(&'src str),
    Number(usize),
    Reference(usize),
    Sym(Symbol),
    Comment(&'src str),
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
            Token::Number(n) => write!(f, "{n}"),
            Token::Reference(r) => write!(f, "#{r}"),
            Token::Sym(s) => write!(f, "{s}"),
            Token::Comment(c) => write!(f, "/*{c}*/"),
            Token::StartTag => write!(f, "ISO-10303-21"),
            Token::EndTag => write!(f, "END-ISO-10303-21"),
        }
    }
}

impl<'src> Token<'src> {
    fn parser<E: ParserExtra<'src, &'src str>>(
    ) -> impl Parser<'src, &'src str, Token<'src>, E> + Copy {
        choice((
            Self::parse_reference(),
            Self::step_identifier(),
            Symbol::parse_symbol().map(Self::Sym),
            Self::parse_string(),
        ))
    }

    fn parse_reference<E: ParserExtra<'src, &'src str>>(
    ) -> impl Parser<'src, &'src str, Token<'src>, E> + Copy {
        just("#")
            .ignore_then(
                any()
                    .filter(|c: &char| c.is_numeric())
                    .repeated()
                    .to_slice(),
            )
            .map(|s: &str| Self::Reference(s.parse::<usize>().unwrap()))
    }

    fn step_identifier<E: ParserExtra<'src, &'src str>>(
    ) -> impl Parser<'src, &'src str, Token<'src>, E> + Copy {
        let step_identifier = any()
            .filter(|c: &char| c.is_ascii_uppercase() || c.is_numeric())
            .then(
                any()
                    .filter(|c: &char| {
                        c.is_ascii_uppercase() || *c == '-' || *c == '_' || c.is_numeric()
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
    fn comment() -> impl Parser<'src, &'src str, Token<'src>, extra::Err<Rich<'src, char>>> {
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

    pub fn lexer(
    ) -> impl Parser<'src, &'src str, Vec<Spanned<Token<'src>>>, extra::Err<Rich<'src, char, Span>>>
    {
        Self::parser()
            .map_with(|tok, e| (tok, e.span()).into())
            .padded()
            .padded_by(Self::comment().or_not())
            .recover_with(skip_then_retry_until(any().ignored(), end()))
            .repeated()
            .collect()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_sym() {
        run_test("&", vec![(Token::Sym(Symbol::Amp), 0..1).into()]);
        run_test(
            "& /* Hello World! */ &",
            vec![
                (Token::Sym(Symbol::Amp), 0..1).into(),
                (Token::Sym(Symbol::Amp), 21..22).into(),
            ],
        );
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
        let s = "ISO-10303-21;
        HEADER;
        FILE_DESCRIPTION(
        /* description */ ('A minimal AP214 example with a single part'),
        /* implementation_level */ '2;1');
        FILE_NAME(
        /* name */ 'demo',
        /* time_stamp */ '2003-12-27T11:57:53',
        /* author */ ('Lothar Klein'),
        /* organization */ ('LKSoft'),
        /* preprocessor_version */ ' ',
        /* originating_system */ 'IDA-STEP',
        /* authorization */ ' ');
        FILE_SCHEMA (('AUTOMOTIVE_DESIGN { 1 0 10303 214 2 1 1}'));
        ENDSEC;
        DATA;
        #10=ORGANIZATION('O0001','LKSoft','company');
        #11=PRODUCT_DEFINITION_CONTEXT('part definition',#12,'manufacturing');
        #12=APPLICATION_CONTEXT('mechanical design');
        #13=APPLICATION_PROTOCOL_DEFINITION('','automotive_design',2003,#12);
        #14=PRODUCT_DEFINITION('0',$,#15,#11);
        #15=PRODUCT_DEFINITION_FORMATION('1',$,#16);
        #16=PRODUCT('A0001','Test Part 1','',(#18));
        #17=PRODUCT_RELATED_PRODUCT_CATEGORY('part',$,(#16));
        #18=PRODUCT_CONTEXT('',#12,'');
        #19=APPLIED_ORGANIZATION_ASSIGNMENT(#10,#20,(#16));
        #20=ORGANIZATION_ROLE('id owner');
        ENDSEC;
        END-ISO-10303-21;";

        use super::Symbol::*;
        use super::Token::*;

        run_test(
            s,
            vec![
                (StartTag, 0..12).into(),
                (Sym(Sem), 12..13).into(),
                (Header, 22..28).into(),
                (Sym(Sem), 28..29).into(),
                (Identifier("FILE_DESCRIPTION"), 38..54).into(),
                (Sym(BrO), 54..55).into(),
                (Sym(BrO), 82..83).into(),
                (
                    String("A minimal AP214 example with a single part"),
                    83..127,
                )
                    .into(),
                (Sym(BrC), 127..128).into(),
                (Sym(Com), 128..129).into(),
                (String("2;1"), 165..170).into(),
                (Sym(BrC), 170..171).into(),
                (Sym(Sem), 171..172).into(),
                (Identifier("FILE_NAME"), 181..190).into(),
                (Sym(BrO), 190..191).into(),
                (String("demo"), 211..217).into(),
                (Sym(Com), 217..218).into(),
                (String("2003-12-27T11:57:53"), 244..265).into(),
                (Sym(Com), 265..266).into(),
                (Sym(BrO), 288..289).into(),
                (String("Lothar Klein"), 289..303).into(),
                (Sym(BrC), 303..304).into(),
                (Sym(Com), 304..305).into(),
                (Sym(BrO), 333..334).into(),
                (String("LKSoft"), 334..342).into(),
                (Sym(BrC), 342..343).into(),
                (Sym(Com), 343..344).into(),
                (String(" "), 380..383).into(),
                (Sym(Com), 383..384).into(),
                (String("IDA-STEP"), 418..428).into(),
                (Sym(Com), 428..429).into(),
                (String(" "), 458..461).into(),
                (Sym(BrC), 461..462).into(),
                (Sym(Sem), 462..463).into(),
                (Identifier("FILE_SCHEMA"), 472..483).into(),
                (Sym(BrO), 484..485).into(),
                (Sym(BrO), 485..486).into(),
                (String("AUTOMOTIVE_DESIGN { 1 0 10303 214 2 1 1}"), 486..528).into(),
                (Sym(BrC), 528..529).into(),
                (Sym(BrC), 529..530).into(),
                (Sym(Sem), 530..531).into(),
                (Endsec, 540..546).into(),
                (Sym(Sem), 546..547).into(),
                (Data, 556..560).into(),
                (Sym(Sem), 560..561).into(),
                (Reference(10), 570..573).into(),
                (Sym(Eq), 573..574).into(),
                (Identifier("ORGANIZATION"), 574..586).into(),
                (Sym(BrO), 586..587).into(),
                (String("O0001"), 587..594).into(),
                (Sym(Com), 594..595).into(),
                (String("LKSoft"), 595..603).into(),
                (Sym(Com), 603..604).into(),
                (String("company"), 604..613).into(),
                (Sym(BrC), 613..614).into(),
                (Sym(Sem), 614..615).into(),
                (Reference(11), 624..627).into(),
                (Sym(Eq), 627..628).into(),
                (Identifier("PRODUCT_DEFINITION_CONTEXT"), 628..654).into(),
                (Sym(BrO), 654..655).into(),
                (String("part definition"), 655..672).into(),
                (Sym(Com), 672..673).into(),
                (Reference(12), 673..676).into(),
                (Sym(Com), 676..677).into(),
                (String("manufacturing"), 677..692).into(),
                (Sym(BrC), 692..693).into(),
                (Sym(Sem), 693..694).into(),
                (Reference(12), 703..706).into(),
                (Sym(Eq), 706..707).into(),
                (Identifier("APPLICATION_CONTEXT"), 707..726).into(),
                (Sym(BrO), 726..727).into(),
                (String("mechanical design"), 727..746).into(),
                (Sym(BrC), 746..747).into(),
                (Sym(Sem), 747..748).into(),
                (Reference(13), 757..760).into(),
                (Sym(Eq), 760..761).into(),
                (Identifier("APPLICATION_PROTOCOL_DEFINITION"), 761..792).into(),
                (Sym(BrO), 792..793).into(),
                (String(""), 793..795).into(),
                (Sym(Com), 795..796).into(),
                (String("automotive_design"), 796..815).into(),
                (Sym(Com), 815..816).into(),
                (Identifier("2003"), 816..820).into(),
                (Sym(Com), 820..821).into(),
                (Reference(12), 821..824).into(),
                (Sym(BrC), 824..825).into(),
                (Sym(Sem), 825..826).into(),
                (Reference(14), 835..838).into(),
                (Sym(Eq), 838..839).into(),
                (Identifier("PRODUCT_DEFINITION"), 839..857).into(),
                (Sym(BrO), 857..858).into(),
                (String("0"), 858..861).into(),
                (Sym(Com), 861..862).into(),
                (Sym(Dol), 862..863).into(),
                (Sym(Com), 863..864).into(),
                (Reference(15), 864..867).into(),
                (Sym(Com), 867..868).into(),
                (Reference(11), 868..871).into(),
                (Sym(BrC), 871..872).into(),
                (Sym(Sem), 872..873).into(),
                (Reference(15), 882..885).into(),
                (Sym(Eq), 885..886).into(),
                (Identifier("PRODUCT_DEFINITION_FORMATION"), 886..914).into(),
                (Sym(BrO), 914..915).into(),
                (String("1"), 915..918).into(),
                (Sym(Com), 918..919).into(),
                (Sym(Dol), 919..920).into(),
                (Sym(Com), 920..921).into(),
                (Reference(16), 921..924).into(),
                (Sym(BrC), 924..925).into(),
                (Sym(Sem), 925..926).into(),
                (Reference(16), 935..938).into(),
                (Sym(Eq), 938..939).into(),
                (Identifier("PRODUCT"), 939..946).into(),
                (Sym(BrO), 946..947).into(),
                (String("A0001"), 947..954).into(),
                (Sym(Com), 954..955).into(),
                (String("Test Part 1"), 955..968).into(),
                (Sym(Com), 968..969).into(),
                (String(""), 969..971).into(),
                (Sym(Com), 971..972).into(),
                (Sym(BrO), 972..973).into(),
                (Reference(18), 973..976).into(),
                (Sym(BrC), 976..977).into(),
                (Sym(BrC), 977..978).into(),
                (Sym(Sem), 978..979).into(),
                (Reference(17), 988..991).into(),
                (Sym(Eq), 991..992).into(),
                (Identifier("PRODUCT_RELATED_PRODUCT_CATEGORY"), 992..1024).into(),
                (Sym(BrO), 1024..1025).into(),
                (String("part"), 1025..1031).into(),
                (Sym(Com), 1031..1032).into(),
                (Sym(Dol), 1032..1033).into(),
                (Sym(Com), 1033..1034).into(),
                (Sym(BrO), 1034..1035).into(),
                (Reference(16), 1035..1038).into(),
                (Sym(BrC), 1038..1039).into(),
                (Sym(BrC), 1039..1040).into(),
                (Sym(Sem), 1040..1041).into(),
                (Reference(18), 1050..1053).into(),
                (Sym(Eq), 1053..1054).into(),
                (Identifier("PRODUCT_CONTEXT"), 1054..1069).into(),
                (Sym(BrO), 1069..1070).into(),
                (String(""), 1070..1072).into(),
                (Sym(Com), 1072..1073).into(),
                (Reference(12), 1073..1076).into(),
                (Sym(Com), 1076..1077).into(),
                (String(""), 1077..1079).into(),
                (Sym(BrC), 1079..1080).into(),
                (Sym(Sem), 1080..1081).into(),
                (Reference(19), 1090..1093).into(),
                (Sym(Eq), 1093..1094).into(),
                (Identifier("APPLIED_ORGANIZATION_ASSIGNMENT"), 1094..1125).into(),
                (Sym(BrO), 1125..1126).into(),
                (Reference(10), 1126..1129).into(),
                (Sym(Com), 1129..1130).into(),
                (Reference(20), 1130..1133).into(),
                (Sym(Com), 1133..1134).into(),
                (Sym(BrO), 1134..1135).into(),
                (Reference(16), 1135..1138).into(),
                (Sym(BrC), 1138..1139).into(),
                (Sym(BrC), 1139..1140).into(),
                (Sym(Sem), 1140..1141).into(),
                (Reference(20), 1150..1153).into(),
                (Sym(Eq), 1153..1154).into(),
                (Identifier("ORGANIZATION_ROLE"), 1154..1171).into(),
                (Sym(BrO), 1171..1172).into(),
                (String("id owner"), 1172..1182).into(),
                (Sym(BrC), 1182..1183).into(),
                (Sym(Sem), 1183..1184).into(),
                (Endsec, 1193..1199).into(),
                (Sym(Sem), 1199..1200).into(),
                (EndTag, 1209..1225).into(),
                (Sym(Sem), 1225..1226).into(),
            ],
        );
    }

    fn run_test(src: &str, cmp: Vec<Spanned<Token>>) {
        let (tokens, errs) = Token::lexer().parse(src).into_output_errors();

        println!("{:?}", tokens);
        println!("{:?}", errs);

        assert!(errs.is_empty());
        assert_eq!(tokens, Some(cmp));

        assert!(errs.is_empty());
    }
}
