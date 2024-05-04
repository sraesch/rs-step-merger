use std::io::{BufRead, BufReader, Read};

use log::trace;

use crate::{Error, Result};

use super::{StepData, StepEntry};

use chumsky::prelude::*;

/// A reader for a STEP file.
pub struct STEPReader<R: Read> {
    /// The lines of the STEP file.
    reader: BufReader<R>,

    /// The ISO string of the STEP file.
    iso: String,
}

impl<R: Read> STEPReader<R> {
    pub fn new(reader: R) -> STEPReader<R> {
        let reader = BufReader::new(reader);

        STEPReader {
            reader,
            iso: String::new(),
        }
    }

    /// Reads the STEP file and returns the step data.
    pub fn read(mut self) -> Result<StepData> {
        self.read_header()?;
        Ok(self.into_step_data())
    }

    /// Transforms the reader into the step data.
    fn into_step_data(self) -> StepData {
        StepData::new(self.iso)
    }

    /// Reads the header of the STEP file.
    fn read_header(&mut self) -> Result<()> {
        // read iso string
        let iso: String = self.read_next_line()?.trim().to_owned();
        if !iso.starts_with("ISO") || !iso.ends_with(';') {
            return Err(Error::IO(format!("Invalid ISO string, got '{}'", iso)));
        }

        self.iso = iso[..iso.len() - 1].to_owned();
        trace!("ISO string: {}", self.iso);

        self.seek_next_line_entry("HEADER;")?;

        Ok(())
    }

    /// Seeks the next line which consists of the given keyword.
    ///
    /// # Arguments
    /// * `keyword` - The keyword to seek.
    fn seek_next_line_entry(&mut self, keyword: &str) -> Result<()> {
        // search for keyword
        loop {
            let line = self.read_next_line()?;
            if line.trim() == keyword {
                break;
            }
        }

        Ok(())
    }

    /// Reads the next line from the STEP file.
    /// Also increments the line index.
    #[inline]
    fn read_next_line(&mut self) -> Result<String> {
        let mut line = String::new();
        let ret = self
            .reader
            .read_line(&mut line)
            .map_err(|e| Error::IO(format!("Failed to read line: {}", e)))?;

        if ret == 0 {
            return Err(Error::IO("End of file reached".to_owned()));
        }

        // map none to error
        Ok(line)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct StepHeader {
    pub iso: String,
    pub implementation_level: String,
    pub protocol: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum ParsedStep {
    Header(StepHeader),
    Entry(StepEntry),
    Data(Vec<StepEntry>),
    Step(StepHeader, Vec<StepEntry>),
}

fn parser() -> impl Parser<char, ParsedStep, Error = Simple<char>> {
    recursive(|value| {
        // The parser for comments and space which we can ignore.
        let comment = just("/*").then(take_until(just("*/"))).padded();
        let ignore = comment.repeated();

        // The parser for the initial ISO string.
        let iso = just("ISO-")
            .ignore_then(filter(|c| *c != ';').repeated())
            .then_ignore(just(';'))
            .collect::<String>()
            .padded()
            .padded_by(ignore);

        let str = just('\'')
            .ignore_then(filter(|c| *c != '\'').repeated())
            .then_ignore(just('\''))
            .collect::<String>()
            .padded()
            .padded_by(ignore);

        // The parser for a list of strings in brackets, i.e. ('Foobar') or ('adasd', 'asdasd').
        let str_brackets = str
            .chain(just(',').ignore_then(str).repeated())
            .or_not()
            .flatten()
            .delimited_by(just('('), just(')'))
            .labelled("array");

        // The parser for the file description.
        let file_description = text::keyword("FILE_DESCRIPTION")
            .ignore_then(
                str_brackets
                    .then_ignore(just(','))
                    .ignore_then(str)
                    .delimited_by(just('('), just(')'))
                    .padded()
                    .padded_by(ignore)
                    .then_ignore(just(';')),
            )
            .padded()
            .padded_by(ignore)
            .labelled("file_description");

        // The the parser for the file name.
        let file_name = text::keyword("FILE_NAME")
            .ignore_then(
                str.then_ignore(just(',')) // name
                    .then_ignore(str) // date
                    .then_ignore(just(','))
                    .ignore_then(str_brackets) // author
                    .then_ignore(just(','))
                    .ignore_then(str_brackets) // organization
                    .then_ignore(just(','))
                    .ignore_then(str) // preprocessor_version
                    .then_ignore(just(','))
                    .ignore_then(str) // originating_system
                    .then_ignore(just(','))
                    .ignore_then(str) // authorization
                    .delimited_by(just('('), just(')'))
                    .padded()
                    .padded_by(ignore)
                    .then_ignore(just(';')),
            )
            .padded()
            .padded_by(ignore)
            .labelled("file_name");

        let file_schema = text::keyword("FILE_SCHEMA")
            .ignore_then(
                str_brackets
                    .delimited_by(just('('), just(')'))
                    .padded()
                    .padded_by(ignore)
                    .then_ignore(just(';')),
            )
            .padded()
            .padded_by(ignore)
            .labelled("file_schema");

        // The parser for the header section.
        let header_section = text::keyword("HEADER")
            .padded()
            .padded_by(ignore)
            .ignore_then(just(';'))
            .ignore_then(file_description)
            .then_ignore(file_name)
            .then(file_schema)
            .padded()
            .padded_by(ignore)
            .then_ignore(text::keyword("ENDSEC"))
            .padded()
            .padded_by(ignore)
            .then_ignore(just(';'))
            .labelled("header_section");

        // The parser for the full header information of a STEP file.
        let header = iso
            .then(header_section)
            .map(|(iso, (implementation_level, protocol))| {
                ParsedStep::Header(StepHeader {
                    iso,
                    implementation_level,
                    protocol,
                })
            })
            .padded()
            .padded_by(ignore)
            .labelled("header");

        header
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reading() {
        let data = r#"

        ISO-10303-21;
        HEADER;
        FILE_DESCRIPTION(('CTC-02 geometry with PMI representation and/or presentation','from the NIST MBE PMI Validation and Conformance Testing Project'),'2;1');
        FILE_NAME('nist_ctc_02_asme1_ap203.stp','2017-03-10T12:15:07-07:00',(''),(''),'','','');
        FILE_SCHEMA (('AP203_CONFIGURATION_CONTROLLED_3D_DESIGN_OF_MECHANICAL_PARTS_AND_ASSEMBLIES_MIM_LF { 1 0 10303 403 2 1 2}'));
        ENDSEC;

        "#;

        let result = parser().parse(data);
        assert!(result.is_ok(), "Failed with {:?}", result);
        let parsed_step = result.unwrap();
        assert_eq!(
            parsed_step,
            ParsedStep::Header(StepHeader{
                iso: "10303-21".to_owned(),
                implementation_level: "2;1".to_owned(),
                protocol: vec!["AP203_CONFIGURATION_CONTROLLED_3D_DESIGN_OF_MECHANICAL_PARTS_AND_ASSEMBLIES_MIM_LF { 1 0 10303 403 2 1 2}".to_owned()],
            })
        );
    }
}
