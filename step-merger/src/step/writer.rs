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
