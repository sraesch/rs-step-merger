mod reader;
mod writer;

use std::{fs::File, ops::Range, path::Path};

use chumsky::chain::Chain;

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
    implementation_level: String,

    /// A list of the protocols used in the STEP file.
    protocol: Vec<String>,

    /// The entries in the STEP file.
    entries: Vec<StepEntry>,

    /// The range of the ids in the STEP file.
    id_range: Range<u64>,
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
            id_range: 0..0,
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
            step_data.set_entries(body);

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
        // Update the id range
        let id = entry.get_id();

        if self.id_range.is_empty() {
            self.id_range = id..(id + 1);
        } else {
            self.id_range.start = self.id_range.start.min(id);
            self.id_range.end = self.id_range.end.max(id + 1);
        }

        self.entries.push(entry);
    }

    /// Sets the entries of the step data and takes the ownership of the entries.
    ///
    /// # Arguments
    /// * `entries` - The entries to be set.
    pub fn set_entries(&mut self, entries: Vec<StepEntry>) {
        self.entries = entries;

        if let Some(first_entry) = self.entries.first() {
            let first_id = first_entry.get_id();
            let (r0, r1) = self
                .entries
                .iter()
                .fold((first_id, first_id), |(r0, r1), s| {
                    (r0.min(s.get_id()), r1.max(s.get_id()))
                });

            self.id_range = r0..(r1 + 1);
        } else {
            self.id_range = 0..0;
        }
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

    /// Returns the range of the ids in the STEP file.
    pub fn get_id_range(&self) -> Range<u64> {
        self.id_range.clone()
    }
}