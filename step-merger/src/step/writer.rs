use std::io::Write;

use super::StepData;

use crate::Result;

/// Writes the given step data to the writer.
///
/// # Arguments
/// * `writer` - The writer to write to.
/// * `step` - The step data to write.
/// * `filename` - The filename string to set in the header.
pub fn write_step<W: Write>(writer: &mut W, step: &StepData, filename: &str) -> Result<()> {
    writeln!(writer, "ISO-{};\n", step.get_iso())?;
    writeln!(writer, "HEADER;")?;
    writeln!(
        writer,
        "FILE_DESCRIPTION((''), '{}');",
        step.get_implementation_level()
    )?;

    let current_date: String = chrono::Local::now().to_rfc3339();
    writeln!(
        writer,
        "FILE_NAME('{}', '{}', (''), (''), 'step-merger', '', '');",
        filename, current_date
    )?;

    writeln!(
        writer,
        "FILE_SCHEMA(('{}'));\n",
        step.get_protocol().join("'), ('")
    )?;

    writeln!(writer, "ENDSEC;\n")?;

    writeln!(writer, "DATA;")?;
    for entry in step.get_entries() {
        writeln!(writer, "#{}={};", entry.id, entry.definition)?;
    }

    writeln!(writer, "ENDSEC;\n")?;

    writeln!(writer, "END-ISO-{};", step.get_iso())?;

    Ok(())
}

#[cfg(test)]
mod test {
    use std::{io::BufWriter, str::FromStr};

    use crate::step::{self, StepEntry};

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
