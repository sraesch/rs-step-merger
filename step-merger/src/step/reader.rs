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

        let str = just('\'')
            .ignore_then(filter(|c| *c != '\'').repeated())
            .then_ignore(just('\''))
            .collect::<String>()
            .padded()
            .padded_by(ignore);

        let int = text::int(10)
            .map(|s: String| s.parse::<u64>().unwrap())
            .padded();

        // The parser for a list of strings in brackets, i.e. ('Foobar') or ('adasd', 'asdasd').
        let str_brackets = str
            .chain(just(',').ignore_then(str).repeated())
            .or_not()
            .flatten()
            .delimited_by(just('('), just(')'))
            .labelled("array");

        // The parser for the initial ISO string.
        let iso = just("ISO-")
            .ignore_then(filter(|c| *c != ';').repeated())
            .then_ignore(just(';'))
            .collect::<String>()
            .padded()
            .padded_by(ignore);

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

        // The parser for the content of a STEP entry.
        let step_entry_content = filter(|c| *c != ';')
            .repeated()
            .collect::<String>()
            .then_ignore(just(';'))
            .padded()
            .padded_by(ignore);

        // The parser for a STEP entry id.
        let step_entry_id = just('#')
            .padded()
            .padded_by(ignore)
            .ignore_then(int)
            .padded()
            .padded_by(ignore);

        // The parser for a STEP entry.
        let step_entry = step_entry_id
            .then_ignore(just('='))
            .then(step_entry_content)
            .padded()
            .padded_by(ignore)
            .map(|(id, content)| StepEntry::new(id, &content.to_owned()))
            .labelled("step_entry");

        let step_entries = step_entry
            .chain(step_entry.repeated())
            .or_not()
            .flatten()
            .map(ParsedStep::Data)
            .labelled("step_entries");

        // The parser for the data section of a STEP file.
        let data = text::keyword("DATA")
            .padded()
            .padded_by(ignore)
            .ignore_then(just(';'))
            .ignore_then(step_entries)
            .padded()
            .padded_by(ignore)
            .then_ignore(text::keyword("ENDSEC"))
            .padded()
            .padded_by(ignore)
            .then_ignore(just(';'))
            .labelled("data");

        // The parser for the full STEP file.
        let step = header
            .then(data)
            .map(|(header, data)| {
                // extract the header from the data
                let header = if let ParsedStep::Header(header) = header {
                    header
                } else {
                    unreachable!()
                };

                // extract the data from the data
                let data = if let ParsedStep::Data(data) = data {
                    data
                } else {
                    unreachable!()
                };

                ParsedStep::Step(header, data)
            })
            .padded()
            .padded_by(ignore)
            .labelled("step");

        step
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

        DATA;
        #10=CONSTRUCTIVE_GEOMETRY_REPRESENTATION_RELATIONSHIP('supplemental geometry','',#376,#11);
        #11=CONSTRUCTIVE_GEOMETRY_REPRESENTATION('supplemental geometry',(#10644,#10645,#10646,#10647,#10648),#46150);
        #12=GEOMETRICALLY_BOUNDED_WIREFRAME_SHAPE_REPRESENTATION('nist_ctc_02_asme1-None',(#10532),#46150);
        #13=PROPERTY_DEFINITION_REPRESENTATION(#17,#15);
        #14=PROPERTY_DEFINITION_REPRESENTATION(#18,#16);
        #15=REPRESENTATION('',(#19),#46150);
        #16=REPRESENTATION('',(#20),#46150);
        ENDSEC;

        END-ISO-10303-21;
            "#;

        let result = parser().parse(data);
        assert!(result.is_ok(), "Failed with {:?}", result);
        let parsed_step = result.unwrap();

        if let ParsedStep::Step(header, body) = parsed_step {
            assert_eq!(header.iso, "10303-21");
            assert_eq!(header.implementation_level, "2;1");
            assert_eq!(header.protocol.len(), 1);
            assert_eq!(header.protocol[0], "AP203_CONFIGURATION_CONTROLLED_3D_DESIGN_OF_MECHANICAL_PARTS_AND_ASSEMBLIES_MIM_LF { 1 0 10303 403 2 1 2}");

            assert_eq!(body.len(), 7);

            for (entry, id) in body.iter().zip(10..) {
                assert_eq!(entry.get_id(), id);
            }

            assert_eq!(body[0].get_definition(), "CONSTRUCTIVE_GEOMETRY_REPRESENTATION_RELATIONSHIP('supplemental geometry','',#376,#11)");
            assert_eq!(body[1].get_definition(), "CONSTRUCTIVE_GEOMETRY_REPRESENTATION('supplemental geometry',(#10644,#10645,#10646,#10647,#10648),#46150)");
            assert_eq!(body[2].get_definition(), "GEOMETRICALLY_BOUNDED_WIREFRAME_SHAPE_REPRESENTATION('nist_ctc_02_asme1-None',(#10532),#46150)");
            assert_eq!(
                body[3].get_definition(),
                "PROPERTY_DEFINITION_REPRESENTATION(#17,#15)"
            );
            assert_eq!(
                body[4].get_definition(),
                "PROPERTY_DEFINITION_REPRESENTATION(#18,#16)"
            );
            assert_eq!(body[5].get_definition(), "REPRESENTATION('',(#19),#46150)");
            assert_eq!(body[6].get_definition(), "REPRESENTATION('',(#20),#46150)");
        } else {
            panic!("Parsed result is not a step file");
        }
    }
}
