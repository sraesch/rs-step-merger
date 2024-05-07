mod parser;
pub mod ref_iter;
mod writer;

use std::{fs::File, io::Cursor, ops::Range, path::Path, str::FromStr};

use crate::{Error, Result};

pub use parser::STEPParser;
pub use writer::StepWriter;

/// A single entry in the STEP file.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct StepEntry {
    /// The id of the entry, must be unique.
    id: u64,

    /// The definition string of the entry.
    definition: String,
}

/// Internal structure for different modes during parsing the references.
#[derive(PartialEq)]
enum Mode {
    Definition,
    Reference,
    String,
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

    /// Updates the references in the step data using the given function.
    ///
    /// # Arguments
    /// * `f` - The function to update the references. Must be a strictly monotonic function.
    pub fn update_references(&mut self, f: impl Fn(u64) -> u64) {
        // update my own id
        self.id = f(self.id);

        // update the references in the definition
        let mut new_definition = String::new();
        let mut mode = Mode::Definition;
        let mut buffer = String::new();
        for c in self.definition.chars() {
            match mode {
                Mode::Definition => {
                    if c == '#' {
                        mode = Mode::Reference;
                    } else if c == '\'' {
                        mode = Mode::String;
                        new_definition.push(c);
                    } else {
                        new_definition.push(c);
                    }
                }
                Mode::Reference => {
                    if c.is_ascii_digit() {
                        buffer.push(c);
                    } else {
                        let id = buffer.parse::<u64>().unwrap();
                        buffer.clear();
                        let new_id = f(id);
                        new_definition.push('#');
                        new_definition.push_str(&new_id.to_string());

                        if c == '\'' {
                            mode = Mode::String;
                            new_definition.push(c);
                        } else if c == '#' {
                            mode = Mode::Reference;
                        } else {
                            mode = Mode::Definition;
                            new_definition.push(c);
                        }
                    }
                }
                Mode::String => {
                    if c == '\'' {
                        mode = Mode::Definition;
                    }

                    new_definition.push(c);
                }
            }
        }

        self.definition = new_definition;
    }

    /// Returns a list of all references in the definition excluding the own id.
    pub fn get_references(&self) -> Vec<u64> {
        // update the references in the definition
        let mut mode = Mode::Definition;
        let mut buffer = String::new();
        let mut result = Vec::new();

        for c in self.definition.chars() {
            match mode {
                Mode::Definition => {
                    if c == '#' {
                        mode = Mode::Reference;
                    } else if c == '\'' {
                        mode = Mode::String;
                    }
                }
                Mode::Reference => {
                    if c.is_ascii_digit() {
                        buffer.push(c);
                    } else {
                        let id = buffer.parse::<u64>().unwrap();
                        buffer.clear();
                        result.push(id);

                        if c == '\'' {
                            mode = Mode::String;
                        } else if c == '#' {
                            mode = Mode::Reference;
                        } else {
                            mode = Mode::Definition;
                        }
                    }
                }
                Mode::String => {
                    if c == '\'' {
                        mode = Mode::Definition;
                    }
                }
            }
        }

        result
    }
}

/// The data of a STEP file.
pub struct StepData {
    /// The entries in the STEP file.
    entries: Vec<StepEntry>,

    /// The range of the ids in the STEP file.
    id_range: Range<u64>,
}

impl StepData {
    /// Creates a new step data with the given ISO string.
    pub fn new() -> StepData {
        StepData {
            entries: Vec::new(),
            id_range: 0..0,
        }
    }

    /// Reads the step data from the given file.
    ///
    /// # Arguments
    /// * `path` - The path to the STEP file.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<StepData> {
        let step_parser = STEPParser::new(File::open(path)?)?;

        let mut entries = Vec::new();
        for entry in step_parser {
            entries.push(entry?);
        }

        let mut step_data = StepData::new();
        step_data.set_entries(entries);

        Ok(step_data)
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

    /// Updates the references in the step data.
    ///
    /// #@ Arguments
    /// * `f` - The function to update the references. Must be a strictly monotonic function.
    pub fn update_reference(&mut self, f: impl Fn(u64) -> u64) {
        for entry in self.entries.iter_mut() {
            entry.update_references(&f);
        }

        self.id_range = f(self.id_range.start)..f(self.id_range.end);
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

    /// Returns the entries in the STEP file.
    pub fn get_entries(&self) -> &[StepEntry] {
        &self.entries
    }

    /// Returns the range of the ids in the STEP file.
    pub fn get_id_range(&self) -> Range<u64> {
        self.id_range.clone()
    }
}

impl FromStr for StepData {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let reader = Cursor::new(s.as_bytes());
        let parsed_step = STEPParser::new(reader)?;

        let mut entries = Vec::new();
        for entry in parsed_step {
            entries.push(entry?);
        }

        let mut step_data = StepData::new();
        step_data.set_entries(entries);

        Ok(step_data)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_ranges() {
        let mut step_data = StepData::new();

        assert_eq!(step_data.get_id_range(), 0..0);

        step_data.add_entry(StepEntry::new(1, ""));
        assert_eq!(step_data.get_id_range(), 1..2);

        step_data.add_entry(StepEntry::new(3, ""));
        assert_eq!(step_data.get_id_range(), 1..4);

        step_data.add_entry(StepEntry::new(2, ""));
        assert_eq!(step_data.get_id_range(), 1..4);

        step_data.add_entry(StepEntry::new(4, ""));
        assert_eq!(step_data.get_id_range(), 1..5);
    }

    #[test]
    fn test_update_reference_simple() {
        let f = |id| id + 1;

        let mut entry = StepEntry::new(1, "IFCFOO('FOO', #2);");
        entry.update_references(f);
        assert_eq!(entry.get_id(), 2);
        assert_eq!(entry.get_definition(), "IFCFOO('FOO', #3);");

        let mut entry = StepEntry::new(1, "IFCFOO('FOO', #2#3);");
        entry.update_references(f);
        assert_eq!(entry.get_id(), 2);
        assert_eq!(entry.get_definition(), "IFCFOO('FOO', #3#4);");
    }

    #[test]
    fn test_update_reference_complex() {
        let f = |id| id + 1000;

        let mut entry = StepEntry::new(1, "(GEOMETRIC_REPRESENTATION_CONTEXT(3)GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#9531))GLOBAL_UNIT_ASSIGNED_CONTEXT((#8,#9,#7))REPRESENTATION_CONTEXT('',''));");
        entry.update_references(f);
        assert_eq!(entry.get_id(), 1001);
        assert_eq!(entry.get_definition(), "(GEOMETRIC_REPRESENTATION_CONTEXT(3)GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#10531))GLOBAL_UNIT_ASSIGNED_CONTEXT((#1008,#1009,#1007))REPRESENTATION_CONTEXT('',''));");
    }

    #[test]
    fn test_get_references_simple() {
        let entry = StepEntry::new(1, "IFCFOO('FOO', #2);");
        assert_eq!(entry.get_references(), vec![2]);

        let entry = StepEntry::new(1, "IFCFOO('FOO', #2#3);");
        assert_eq!(entry.get_references(), vec![2, 3]);
    }

    #[test]
    fn test_get_reference_complex() {
        let entry = StepEntry::new(1, "(GEOMETRIC_REPRESENTATION_CONTEXT(3)GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#9531))GLOBAL_UNIT_ASSIGNED_CONTEXT((#8,#9,#7))REPRESENTATION_CONTEXT('',''));");
        assert_eq!(entry.get_references(), vec![9531, 8, 9, 7]);
    }
}
