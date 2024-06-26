use std::{io::Write, sync::Arc};

use log::debug;

use super::{StepData, StepEntry};

use crate::{Error, Result};

// All std::io::Error instances in this file produce StepFileWrite errors
// Allows to use ? shorthand
impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::StepFileWrite(Arc::new(value))
    }
}

/// Writes the given step data to the writer.
///
/// # Arguments
/// * `writer` - The writer to write to.
/// * `step` - The step data to write.
/// * `filename` - The filename string to set in the header.
pub fn write_step<W: Write>(writer: &mut W, step: &StepData, filename: &str) -> Result<()> {
    let protocol = vec!["AP203_CONFIGURATION_CONTROLLED_3D_DESIGN_OF_MECHANICAL_PARTS_AND_ASSEMBLIES_MIM_LF { 1 0 10303 403 1 1 4 }".to_owned()];
    let mut step_writer = StepWriter::new(writer, "2;1", filename, &protocol)?;

    for entry in step.get_entries() {
        step_writer.write_entry(entry)?;
    }

    step_writer.finalize()?;

    Ok(())
}

/// A step writer for manually writing the step entries. Can be used to stream the
/// step entries to a writer.
pub struct StepWriter<W: Write> {
    /// The underlying writer to write to.
    writer: W,

    /// Indicates if the writer has been finalized.
    /// If finalized, no further entries can be written.
    is_finalized: bool,
}

impl<W: Write> StepWriter<W> {
    /// Creates and initializes a new step writer.
    ///
    /// # Arguments
    /// * `writer` - The writer to write to.
    /// * `implementation_level` - The implementation level string to set in the header.
    /// * `filename` - The filename string to set in the header.
    /// * `protocol` - The protocol strings to set in the header.
    pub fn new(
        writer: W,
        implementation_level: &str,
        filename: &str,
        protocol: &[String],
    ) -> Result<Self> {
        let mut step_writer = StepWriter {
            writer,
            is_finalized: false,
        };
        debug!("Writing step file header...");
        step_writer.write_header(implementation_level, filename, protocol)?;
        debug!("Writing step file header...DONE");

        // initialize data block
        debug!("Start writing data...");
        writeln!(step_writer.writer, "DATA;")?;

        Ok(step_writer)
    }

    /// Writes the given step entry to the writer.
    ///
    /// # Arguments
    /// * `entry` - The step entry to write.
    pub fn write_entry(&mut self, entry: &StepEntry) -> Result<()> {
        assert!(
            !self.is_finalized,
            "Cannot write entry after finalizing the step writer."
        );
        writeln!(self.writer, "#{}={};", entry.id, entry.definition)?;

        Ok(())
    }

    /// Writes the STEP file header for the given header values.
    ///
    /// # Arguments
    /// * `implementation_level` - The implementation level string to set in the header.
    /// * `filename` - The filename string to set in the header.
    /// * `protocol` - The protocol strings to set in the header.
    fn write_header(
        &mut self,
        implementation_level: &str,
        filename: &str,
        protocol: &[String],
    ) -> Result<()> {
        writeln!(self.writer, "ISO-10303-21;\n")?;
        writeln!(self.writer, "HEADER;\n")?;
        writeln!(
            self.writer,
            "FILE_DESCRIPTION((''), '{}');",
            implementation_level
        )?;

        let current_date: String = chrono::Local::now().to_rfc3339();
        writeln!(
            self.writer,
            "FILE_NAME('{}', '{}', (''), (''), 'step-merger', '', '');",
            filename, current_date
        )?;

        writeln!(
            self.writer,
            "FILE_SCHEMA(('{}'));\n",
            protocol.join("'), ('")
        )?;

        writeln!(self.writer, "ENDSEC;\n")?;
        self.writer.flush()?;

        Ok(())
    }

    /// Finalizes the step writer.
    pub fn finalize(&mut self) -> Result<()> {
        debug!("Finalizing step writer...");

        if self.is_finalized {
            return Ok(());
        } else {
            self.is_finalized = true;
        }

        writeln!(self.writer, "ENDSEC;\n")?;
        writeln!(self.writer, "END-ISO-10303-21;")?;
        self.writer.flush()?;

        debug!("Finalizing step writer...DONE");

        Ok(())
    }
}

impl<W: Write> Drop for StepWriter<W> {
    fn drop(&mut self) {
        if let Err(err) = self.finalize() {
            log::error!("Failed to finalize step writer: {}", err);
        }
    }
}

#[cfg(test)]
mod test {
    use std::{io::BufWriter, str::FromStr};

    use crate::step;

    use super::*;

    #[test]
    fn test_writing_simple() {
        let data = include_str!("../../../test_data/minimal-structure.stp");
        let step = step::StepData::from_str(data).unwrap();
        assert_eq!(step.get_entries().len(), 65);

        let mut serialized_data: Vec<u8> = Vec::new();
        {
            let mut writer = BufWriter::new(&mut serialized_data);
            write_step(&mut writer, &step, "minimal-structure.stp").unwrap();
        }

        let serialized_data = String::from_utf8(serialized_data).unwrap();
        let step2 = step::StepData::from_str(&serialized_data).unwrap();
        assert_eq!(step.get_entries().len(), step2.get_entries().len());

        for (entry1, entry2) in step.get_entries().iter().zip(step2.get_entries().iter()) {
            assert_eq!(entry1.id, entry2.id);
            assert_eq!(entry1.definition, entry2.definition);
        }
    }
}
