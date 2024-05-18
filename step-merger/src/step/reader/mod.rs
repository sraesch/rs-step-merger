mod logos_parser;
mod plain_parser;

use std::io::Read;

use crate::Result;

pub use logos_parser::lexer_logos::Token;

use super::StepEntry;

/// A trait for STEP readers.
pub trait STEPReaderTrait<R: Read>: Sized + Iterator<Item = Result<StepEntry>> {
    /// Returns the name of the parser.
    fn get_name(&self) -> &'static str;

    /// Creates a new STEP parser from a reader.
    ///
    /// # Arguments
    /// * `reader` - The reader to parse the STEP-data from.
    fn new(reader: R) -> Result<Self>;
}

pub type STEPReaderPlain<R> = plain_parser::STEPReader<R>;
pub type STEPReaderLogos<R> = logos_parser::STEPReader<R>;

/// A type alias for the default STEP reader.
pub type STEPReader<R> = STEPReaderLogos<R>;
