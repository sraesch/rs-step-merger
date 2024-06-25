use std::io::Cursor;

use step_merger::step::{STEPReaderLogos, STEPReaderPlain, STEPReaderTrait, StepEntry};

fn init_parser<P: STEPReaderTrait<Cursor<&'static str>>>() {
    let input = Cursor::new("ISO-10303-21; DATA;");
    P::new(input).unwrap();

    let input = Cursor::new("ISO-10303-21 ; DATA ;");
    P::new(input).unwrap();

    let input = Cursor::new("ISO-10304-21 ; DATA;");
    assert!(P::new(input).is_err());

    let input = Cursor::new("ISO-10303-21 ; ");
    assert!(P::new(input).is_err());
}

fn read_next_entry1<P: STEPReaderTrait<Cursor<&'static str>>>() {
    let input = Cursor::new("ISO-10303-21; DATA; #1=; ENDSEC;");
    let mut parser = P::new(input).unwrap();

    let entry = parser.next().unwrap().unwrap();
    assert_eq!(entry.get_id(), 1);
    assert_eq!(entry.get_definition(), "");

    assert!(parser.next().is_none());
}

fn read_next_entry2<P: STEPReaderTrait<Cursor<&'static [u8]>>>() {
    let input = Cursor::new(include_bytes!("../../test_data/wiki.stp").as_slice());
    let parser = P::new(input).unwrap();

    let entries: Vec<StepEntry> = parser.into_iter().map(|r| r.unwrap()).collect();
    assert_eq!(entries.len(), 11);

    assert!(entries
        .iter()
        .enumerate()
        .all(|(i, entry)| entry.get_id() == i as u64 + 10));

    assert_eq!(
        entries[0].get_definition(),
        "ORGANIZATION('O0001','LKSoft','company')"
    );
    assert_eq!(
        entries[1].get_definition(),
        "PRODUCT_DEFINITION_CONTEXT('part definition',#12,'manufacturing')"
    );
    assert_eq!(
        entries[2].get_definition(),
        "APPLICATION_CONTEXT('mechanical design')"
    );
    assert_eq!(
        entries[3].get_definition(),
        "APPLICATION_PROTOCOL_DEFINITION('','automotive_design',2003,#12)"
    );
    assert_eq!(
        entries[4].get_definition(),
        "PRODUCT_DEFINITION('0',$,#15,#11)"
    );
    assert_eq!(
        entries[5].get_definition(),
        "PRODUCT_DEFINITION_FORMATION('1',$,#16)"
    );
    assert_eq!(
        entries[6].get_definition(),
        "PRODUCT('A0001','Test Part 1','',(#18))"
    );
    assert_eq!(
        entries[7].get_definition(),
        "PRODUCT_RELATED_PRODUCT_CATEGORY('part',$,(#16))"
    );
    assert_eq!(entries[8].get_definition(), "PRODUCT_CONTEXT('',#12,'')");
    assert_eq!(
        entries[9].get_definition(),
        "APPLIED_ORGANIZATION_ASSIGNMENT(#10,#20,(#16))"
    );
    assert_eq!(
        entries[10].get_definition(),
        "ORGANIZATION_ROLE('id owner')"
    );
}

#[test]
fn test_init_parser_plain() {
    init_parser::<STEPReaderPlain<Cursor<&'static str>>>();
}

#[test]
fn test_init_parser_logos() {
    init_parser::<STEPReaderLogos<Cursor<&'static str>>>();
}

#[test]
fn test_read_next_entry1_plain() {
    read_next_entry1::<STEPReaderPlain<Cursor<&'static str>>>();
}

#[test]
fn test_read_next_entry1_logos() {
    read_next_entry1::<STEPReaderLogos<Cursor<&'static str>>>();
}

#[test]
fn test_read_next_entry2_plain() {
    read_next_entry2::<STEPReaderPlain<Cursor<&'static [u8]>>>();
}

#[test]
fn test_read_next_entry2_logos() {
    read_next_entry2::<STEPReaderLogos<Cursor<&'static [u8]>>>();
}
