use std::collections::{HashMap, HashSet};

use log::error;

use crate::step::StepEntry;

use super::utils::NodeStepIds;

/// The structure to find the root nodes in the step data entries.
/// We have the following entities:
/// * PRODUCT_DEFINITION
/// * SHAPE_REPRESENTATION
/// * PRODUCT_DEFINITION_SHAPE
/// * SHAPE_DEFINITION_REPRESENTATION
/// * NEXT_ASSEMBLY_USAGE_OCCURRENCE
///
/// The references between the entities are as follows:
/// * NEXT_ASSEMBLY_USAGE_OCCURRENCE -> PRODUCT_DEFINITION
/// * PRODUCT_DEFINITION_SHAPE -> PRODUCT_DEFINITION
/// * SHAPE_DEFINITION_REPRESENTATION -> SHAPE_REPRESENTATION
/// * SHAPE_DEFINITION_REPRESENTATION -> PRODUCT_DEFINITION_SHAPE
///
/// We are interested to find the root node which is the node that has no parent, i.e. the
/// product definition where no NEXT_ASSEMBLY_USAGE_OCCURRENCE references it.
/// We then have to return the SHAPE_REPRESENTATION and PRODUCT_DEFINITION_SHAPE ids.
#[derive(Default)]
pub struct FindRootNodes {
    shape_def_rep_to_shape_rep: HashMap<u64, u64>,
    prod_def_shape_to_shape_def_rep: HashMap<u64, u64>,
    prod_def_to_prod_def_shape: Vec<(u64, u64)>,
    prod_def_assembly_occurrences: HashSet<u64>,
}

impl FindRootNodes {
    /// Creates a new instance.
    pub fn new() -> Self {
        FindRootNodes::default()
    }

    /// Adds the given entry to the internal data structure.
    ///
    /// # Arguments
    /// * `entry` - The entry to be added.
    pub fn add_entry(&mut self, entry: &StepEntry) {
        let keyword = Self::extract_keyword(entry.get_definition());

        match keyword {
            "SHAPE_DEFINITION_REPRESENTATION" => {
                let shape_def_rep_id = entry.get_id();
                let references = entry.get_references();
                if references.len() != 2 {
                    error!(
                        "SHAPE_DEFINITION_REPRESENTATION entry with id {} has {} references",
                        shape_def_rep_id,
                        references.len()
                    );
                    return;
                }

                let prod_def_shape_id = references[0];
                let shape_rep_id = references[1];

                self.shape_def_rep_to_shape_rep
                    .insert(shape_def_rep_id, shape_rep_id);
                self.prod_def_shape_to_shape_def_rep
                    .insert(prod_def_shape_id, shape_def_rep_id);
            }
            "PRODUCT_DEFINITION_SHAPE" => {
                let prod_def_shape_id = entry.get_id();
                let references = entry.get_references();
                if references.is_empty() {
                    error!(
                        "PRODUCT_DEFINITION_SHAPE entry with id {} has no references",
                        prod_def_shape_id,
                    );
                    return;
                }

                let prod_def_id = references.last().unwrap();
                self.prod_def_to_prod_def_shape
                    .push((*prod_def_id, prod_def_shape_id));
            }
            "NEXT_ASSEMBLY_USAGE_OCCURRENCE" => {
                let references = entry.get_references();
                if references.len() != 2 {
                    error!(
                        "NEXT_ASSEMBLY_USAGE_OCCURRENCE entry with id {} has {} references",
                        entry.get_id(),
                        references.len()
                    );
                    return;
                }

                let prod_def_id = references[1];
                self.prod_def_assembly_occurrences.insert(prod_def_id);
            }
            _ => {}
        }
    }

    /// Extracts the root nodes based on the collected entries and returns them
    pub fn get_root_nodes(&self) -> Vec<NodeStepIds> {
        let mut result = Vec::new();
        for (prod_def_id, prod_def_shape_id) in self.prod_def_to_prod_def_shape.iter() {
            // skip if the product definition is referenced by an assembly occurrence
            if self.prod_def_assembly_occurrences.contains(prod_def_id) {
                continue;
            }

            // try to find the shape definition representation
            if let Some(shape_def_rep_id) =
                self.prod_def_shape_to_shape_def_rep.get(prod_def_shape_id)
            {
                // try to find the shape representation
                if let Some(shape_rep_id) = self.shape_def_rep_to_shape_rep.get(shape_def_rep_id) {
                    result.push(NodeStepIds {
                        product_definition_id: *prod_def_id,
                        shape_representation_id: *shape_rep_id,
                    });
                } else {
                    error!(
                        "No SHAPE_REPRESENTATION found for SHAPE_DEFINITION_REPRESENTATION {}",
                        shape_def_rep_id
                    );
                }
            } else {
                error!(
                    "No SHAPE_DEFINITION_REPRESENTATION found for PRODUCT_DEFINITION_SHAPE {}",
                    prod_def_shape_id
                );
            }
        }

        result
    }

    /// Extracts the keyword from the given definition
    ///
    /// # Arguments
    /// * `definition` - The definition to extract the keyword from.
    fn extract_keyword(definition: &str) -> &str {
        let definition = definition.trim();

        // find first character that does not belong to the keyword characters
        let keyword_end = definition
            .find(|c: char| !c.is_ascii_uppercase() && c != '_')
            .unwrap_or(definition.len());
        &definition[..keyword_end]
    }
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use crate::step::StepData;

    use super::*;

    #[test]
    fn test_extract_keyword() {
        let s = "PRODUCT_DEFINITION_SHAPE('',#,#);";
        let keyword = FindRootNodes::extract_keyword(s);

        assert_eq!(keyword, "PRODUCT_DEFINITION_SHAPE");

        let s = "  FOOBAR_BLUB( );  ";
        let keyword = FindRootNodes::extract_keyword(s);

        assert_eq!(keyword, "FOOBAR_BLUB");
    }

    #[test]
    fn test_find_root_nodes1() {
        let source = include_str!("../../../test_data/minimal-structure.stp");
        let step_data = StepData::from_str(source).unwrap();

        let entries = step_data.get_entries();
        let mut find_root_nodes = FindRootNodes::new();
        for entry in entries.iter() {
            find_root_nodes.add_entry(entry);
        }

        let root_nodes = find_root_nodes.get_root_nodes();

        assert_eq!(root_nodes.len(), 2);

        assert_eq!(root_nodes[0].product_definition_id, 14);
        assert_eq!(root_nodes[0].shape_representation_id, 19);

        assert_eq!(root_nodes[1].product_definition_id, 2014);
        assert_eq!(root_nodes[1].shape_representation_id, 2019);
    }

    #[test]
    fn test_find_root_nodes2() {
        let source = include_str!("../../../test_data/2-cubes-1-sphere.stp");
        let step_data = StepData::from_str(source).unwrap();

        let entries = step_data.get_entries();
        let mut find_root_nodes = FindRootNodes::new();
        for entry in entries.iter() {
            find_root_nodes.add_entry(entry);
        }

        let root_nodes = find_root_nodes.get_root_nodes();

        assert_eq!(root_nodes.len(), 1);

        assert_eq!(root_nodes[0].product_definition_id, 14);
        assert_eq!(root_nodes[0].shape_representation_id, 31);
    }
}
