use log::{debug, error, trace};

use crate::step::StepEntry;

/// The ids being generated for a node while creating the step data.
#[derive(Debug, Clone, Copy)]
pub struct NodeStepIds {
    pub product_definition_id: u64,
    pub shape_representation_id: u64,
}

/// Extracts all ids for the items defined in MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION entry.
///
/// # Arguments
/// * `entry` - The MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION entry.
/// * `ids` - The list of ids where the extracted ids are added.
pub fn get_ids_from_mechanical_part(entry: &StepEntry, ids: &mut Vec<u64>) {
    let mut entry_ids = entry.get_references();
    if entry_ids.is_empty() {
        error!(
            "No references found in MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION entry"
        );
        return;
    } else {
        entry_ids.pop();
    }

    debug!(
        "MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION got id {} and has {} ids",
        entry.get_id(),
        entry_ids.len()
    );
    trace!("ids: {:?}", entry_ids);

    ids.append(&mut entry_ids);
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_mechanical_design_entry() {
        let s = "MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION('',(#24),#187);";
        let entry = StepEntry::new(1, s);
        let mut ids = Vec::new();

        get_ids_from_mechanical_part(&entry, &mut ids);

        assert_eq!(ids, vec![24]);
    }
}
