use std::io::Read;

use self::parser::Parser;

use crate::{Error, Result};

use super::StepEntry;

mod char_parser;
mod parser;
mod whitespace_parser;

pub struct STEPParser<R: Read> {
    parser: Parser<R>,
    reached_end: bool,
}

impl<R: Read> STEPParser<R> {
    /// Creates a new STEP parser from a reader.
    ///
    /// # Arguments
    /// * `reader` - The reader to read from.
    pub fn new(reader: R) -> Result<Self> {
        let mut step_parser = STEPParser {
            parser: Parser::new(reader),
            reached_end: false,
        };

        step_parser.parse_iso_line()?;
        step_parser.find_data_section()?;

        Ok(step_parser)
    }

    /// Parses the initial ISO String 'ISO-10303-21'.
    fn parse_iso_line(&mut self) -> Result<()> {
        self.parser.skip_whitespace_tokens()?;
        self.parser.read_exact_sequence("ISO-10303-21")?;
        self.parser.skip_whitespace_tokens()?;
        self.parser.read_exact_sequence(";")?;

        Ok(())
    }

    /// Searches for the DATA section.
    fn find_data_section(&mut self) -> Result<()> {
        loop {
            self.parser.skip_whitespace_tokens()?;
            let identifier = self
                .parser
                .read_string(|ch| ch.is_ascii_alphabetic(), false)?;

            if identifier == "DATA" {
                self.parser.skip_whitespace_tokens()?;
                self.parser.read_exact_sequence(";")?;

                break;
            }

            let num_skipped = self.parser.skip_until(|ch| !ch.is_ascii_alphabetic())?;
            if num_skipped == 0 && identifier.is_empty() {
                return Err(Error::InvalidFormat("DATA section not found".to_string()));
            }
        }

        Ok(())
    }

    /// Reads the next STEP entry and returns none if the end of the section is reached.
    /// Otherwise, returns the read STEP entry.
    fn read_next_entry(&mut self) -> Result<Option<StepEntry>> {
        // check if the end of the section is already reached
        if self.reached_end {
            return Ok(None);
        }

        self.parser.skip_whitespace_tokens()?;

        // check if there is some identifier
        let identifier = self
            .parser
            .read_string(|ch| ch.is_ascii_alphabetic(), false)?;

        // check if the read identifier is the end of the section
        if identifier == "ENDSEC" {
            self.reached_end = true;
            self.parser.skip_whitespace_tokens()?;
            self.parser.read_exact_sequence(";")?;

            return Ok(None);
        } else if !identifier.is_empty() {
            return Err(Error::InvalidFormat(format!(
                "Unexpected identifier: {}",
                identifier
            )));
        }

        // no identifier, so there must be a new STEP entry
        self.parser.read_exact_sequence("#")?;
        let id = self.parser.read_u64()?;
        self.parser.skip_whitespace_tokens()?;
        self.parser.read_exact_sequence("=")?;
        self.parser.skip_whitespace_tokens()?;
        let definition = self.parser.read_string(|ch| ch != ';', true)?;
        self.parser.read_exact_sequence(";")?;

        Ok(Some(StepEntry { id, definition }))
    }
}

impl<R: Read> Iterator for STEPParser<R> {
    type Item = Result<StepEntry>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.read_next_entry() {
            Ok(Some(entry)) => Some(Ok(entry)),
            Ok(None) => None,
            Err(err) => Some(Err(err)),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn test_init_parser() {
        let mut input = Cursor::new("ISO-10303-21; DATA;");
        STEPParser::new(&mut input).unwrap();

        let mut input = Cursor::new("ISO-10303-21 ; DATA ;");
        STEPParser::new(&mut input).unwrap();

        let mut input = Cursor::new("ISO-10304-21 ; DATA;");
        assert!(STEPParser::new(&mut input).is_err());

        let mut input = Cursor::new("ISO-10303-21 ; ");
        assert!(STEPParser::new(&mut input).is_err());
    }

    #[test]
    fn test_read_next_entry1() {
        let mut input = Cursor::new("ISO-10303-21; DATA; #1=; ENDSEC;");
        let mut parser = STEPParser::new(&mut input).unwrap();

        let entry = parser.next().unwrap().unwrap();
        assert_eq!(entry.id, 1);
        assert_eq!(entry.definition, "");

        assert!(parser.next().is_none());
    }

    #[test]
    fn test_read_next_entry2() {
        let mut input = Cursor::new(include_bytes!("../../../../test_data/wiki.stp"));
        let parser = STEPParser::new(&mut input).unwrap();

        let entries: Vec<StepEntry> = parser.into_iter().map(|r| r.unwrap()).collect();
        assert_eq!(entries.len(), 11);

        assert!(entries
            .iter()
            .enumerate()
            .all(|(i, entry)| entry.id == i as u64 + 10));

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
}
