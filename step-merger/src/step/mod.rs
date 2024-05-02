mod reader;

use std::{collections::BTreeMap, fs::File, path::Path};

use self::reader::STEPReader;
use crate::Result;

/// A single entry in the STEP file.
pub struct STEPEntry {
    /// The id of the entry, must be unique.
    id: i64,

    /// The definition string of the entry.
    definition: String,
}

impl STEPEntry {
    /// Creates a new STEP entry with the given id and definition.
    ///
    /// # Arguments
    /// * `id` - The id of the entry.
    /// * `definition` - The definition string of the entry.
    pub fn new(id: i64, definition: &str) -> STEPEntry {
        STEPEntry {
            id,
            definition: definition.to_owned(),
        }
    }

    /// Returns the id of the entry.
    pub fn get_id(&self) -> i64 {
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

    entries: BTreeMap<i64, STEPEntry>,
}

impl StepData {
    /// Creates a new step data with the given ISO string.
    ///
    /// # Arguments
    /// * `iso` - The ISO string of the STEP file.
    pub fn new(iso: String) -> StepData {
        StepData {
            iso,
            entries: BTreeMap::new(),
        }
    }

    /// Reads the step data from the given file.
    ///
    /// # Arguments
    /// * `path` - The path to the STEP file.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<StepData> {
        let file = File::open(path)?;
        let reader = STEPReader::new(file);
        let step_data = reader.read()?;

        Ok(step_data)
    }

    /// Adds an entry to the step data.
    ///
    /// # Arguments
    /// * `entry` - The entry to be added.
    pub fn add_entry(&mut self, entry: STEPEntry) {
        self.entries.insert(entry.id, entry);
    }

    /// Returns the ISO string of the STEP file.
    pub fn get_iso(&self) -> &str {
        &self.iso
    }
}
