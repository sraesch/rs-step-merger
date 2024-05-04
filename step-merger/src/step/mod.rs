mod reader;
mod writer;

use std::{fs::File, path::Path};

use crate::{Error, Result};

use self::reader::ParsedStep;

/// A single entry in the STEP file.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct StepEntry {
    /// The id of the entry, must be unique.
    id: u64,

    /// The definition string of the entry.
    definition: String,
}

impl StepEntry {
    /// Creates a new STEP entry with the given id and definition.
    ///
    /// # Arguments
    /// * `id` - The id of the entry.
    /// * `definition` - The definition string of the entry.
    pub fn new(id: u64, definition: &str) -> StepEntry {
        StepEntry {
            id,
            definition: definition.to_owned(),
        }
    }

    /// Returns the id of the entry.
    pub fn get_id(&self) -> u64 {
        self.id
    }

    /// Returns the definition string of the entry.
    pub fn get_definition(&self) -> &str {
        &self.definition
    }
}

/// The data of a STEP file.
pub struct StepData {
    /// The ISO string of the STEP file.
    iso: String,

    /// The implementation level of the STEP file.
    pub implementation_level: String,

    /// A list of the protocols used in the STEP file.
    pub protocol: Vec<String>,

    /// The entries in the STEP file.
    entries: Vec<StepEntry>,
}

impl StepData {
    /// Creates a new step data with the given ISO string.
    ///
    /// # Arguments
    /// * `iso` - The ISO string of the STEP file.
    /// * `implementation_level` - The implementation level of the STEP file.
    /// * `protocol` - A list of the protocols used in the STEP file.
    pub fn new(iso: String, implementation_level: String, protocol: Vec<String>) -> StepData {
        StepData {
            iso,
            implementation_level,
            protocol,
            entries: Vec::new(),
        }
    }

    /// Reads the step data from the given file.
    ///
    /// # Arguments
    /// * `path` - The path to the STEP file.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<StepData> {
        let parsed_step = {
            let file = File::open(path)?;
            ParsedStep::parse(file)?
        };

        if let ParsedStep::Step(header, body) = parsed_step {
            let iso_string = header.iso;
            let implementation_level = header.implementation_level;
            let protocol = header.protocol;

            let mut step_data = StepData::new(iso_string, implementation_level, protocol);
            step_data.entries = body;

            Ok(step_data)
        } else {
            Err(Error::ParsingError("Invalid parsed step".to_owned()))
        }
    }

    /// Writes the step data to the given file.
    ///
    /// # Arguments
    /// * `filename` - The path to the file to write to.
    pub fn to_file<P: AsRef<Path>>(&self, filename: P) -> Result<()> {
        let filename_str: String = filename.as_ref().to_string_lossy().to_string();

        let mut file = File::create(filename)?;
        writer::write_step(&mut file, self, filename_str.as_str())
    }

    /// Adds an entry to the step data.
    ///
    /// # Arguments
    /// * `entry` - The entry to be added.
    pub fn add_entry(&mut self, entry: StepEntry) {
        self.entries.push(entry);
    }

    /// Returns the ISO string of the STEP file.
    pub fn get_iso(&self) -> &str {
        &self.iso
    }

    /// Returns the implementation string.
    pub fn get_implementation_level(&self) -> &str {
        &self.implementation_level
    }

    /// Returns the protocol list.
    pub fn get_protocol(&self) -> &[String] {
        &self.protocol
    }

    /// Returns the entries in the STEP file.
    pub fn get_entries(&self) -> &[StepEntry] {
        &self.entries
    }
}
