use std::{collections::HashMap, path::Path};

use crate::{Assembly, Error, Node, Result};

use log::{debug, error, info};

use super::{StepData, StepEntry};

/// The function consumes the given assembly structure and creates step data consisting of the
/// given assembly structure
///
/// # Arguments
/// * `assembly` - The assembly structure to be merged.
pub fn merge_assembly_structure_to_step(assembly: &Assembly) -> Result<StepData> {
    let mut merger = StepMerger::new(assembly);
    merger.merge()?;

    Ok(merger.step_data)
}

/// The internal step merge operator
struct StepMerger<'a> {
    /// The assembly structure to be merged.
    assembly: &'a Assembly,

    /// The id counter for the step entries.
    id_counter: u64,

    // The serialized step data
    step_data: StepData,

    /// The list of loaded step files and their respective ids
    reference_map: HashMap<String, Option<NodeStepIds>>,

    /// The list of referenced mechanical design entries
    mechanical_design_ids: Vec<u64>,
}

impl<'a> StepMerger<'a> {
    /// Creates a new step merger with the given writer and assembly structure.
    ///
    /// # Arguments
    /// * `writer` - The writer for the merged step file.
    /// * `assembly` - The assembly structure to be merged.
    pub fn new(assembly: &'a Assembly) -> StepMerger<'a> {
        let protocol = vec!["AP203_CONFIGURATION_CONTROLLED_3D_DESIGN_OF_MECHANICAL_PARTS_AND_ASSEMBLIES_MIM_LF { 1 0 10303 403 1 1 4 }".to_owned()];
        let step_data = StepData::new("10303-21".to_owned(), "2;1".to_owned(), protocol);

        StepMerger {
            assembly,
            id_counter: 0,
            step_data,
            reference_map: HashMap::new(),
            mechanical_design_ids: Vec::new(),
        }
    }

    /// Merges the assembly structure into a single monolithic step file.
    pub fn merge(&mut self) -> Result<()> {
        info!("Merging assembly structure into step file...");
        self.create_app_context();

        // create all nodes and collect the node product definition and shape representation ids
        let node_step_ids: Vec<NodeStepIds> = self
            .assembly
            .nodes
            .iter()
            .map(|node| self.create_node(node))
            .collect();

        // create parent-child relations
        for (node, parent_ids) in self.assembly.nodes.iter().zip(node_step_ids.iter()) {
            for child in node.get_children() {
                let child_ids = &node_step_ids[*child];
                let child = &self.assembly.nodes[*child];
                self.create_parent_child_relation(
                    node.get_label(),
                    child.get_label(),
                    *parent_ids,
                    *child_ids,
                );
            }
        }

        // load all referenced step files and add them to the current step data
        for node in self.assembly.nodes.iter() {
            if let Some(link) = node.get_link() {
                if !self.reference_map.contains_key(link) {
                    self.load_and_add_step(link)?;
                }
            }
        }

        debug!("Write mechanical part entries...");
        self.write_mechanical_part_entries();
        debug!("Write mechanical part entries...DONE");

        Ok(())
    }

    /// Returns a new unique id.
    #[inline]
    fn get_new_id(&mut self) -> u64 {
        self.id_counter += 1;
        self.id_counter
    }

    /// Adds a new entry to the step data based on the given definition.
    /// Returns the id of the new entry.
    ///
    /// # Arguments
    /// * `definition` - The definition of the entry.
    #[inline]
    fn add_entry(&mut self, definition: &str) -> u64 {
        let id = self.get_new_id();
        self.step_data.add_entry(StepEntry::new(id, definition));

        id
    }

    /// Creates the application context and protocol definition.
    fn create_app_context(&mut self) {
        let app_id = self.add_entry(
            "APPLICATION_CONTEXT('Configuration controlled 3D designs of mechanical parts and assemblies')",
        );

        assert_eq!(app_id, 1);

        self.add_entry("APPLICATION_PROTOCOL_DEFINITION('international standard', 'configuration_control_3d_design_ed2_mim',2004, #1)",
        );
    }

    /// Loads the given step file and adds the loaded step data to the current step data.
    ///
    /// # Arguments
    /// * `file_path` - The path to the step file to be loaded.
    fn load_and_add_step<P: AsRef<Path>>(&mut self, file_path: P) -> Result<()> {
        info!("Load step file {}...", file_path.as_ref().display());
        let step_data = StepData::from_file(file_path.as_ref())?;
        info!("Load step file {}...DONE", file_path.as_ref().display());

        // find the id for the APPLICATION_CONTEXT entry
        let app_context_id = step_data
            .get_entries()
            .iter()
            .find(|entry| {
                entry
                    .get_definition()
                    .trim_start()
                    .starts_with("APPLICATION_CONTEXT")
            })
            .map(|entry| entry.get_id())
            .ok_or_else(|| {
                Error::InvalidFormat(format!(
                    "No APPLICATION_CONTEXT entry found in step file {}",
                    file_path.as_ref().display()
                ))
            })?;

        // define the update function for the ids to elevate all ids and to change the APPLICATION_CONTEXT id
        let id_offset = self.id_counter;
        let update_id = |id: u64| {
            if id == app_context_id {
                1
            } else {
                id + id_offset
            }
        };

        // add the entries to the current step data
        let mut max_id = 0u64;
        for entry in step_data.get_entries() {
            let definition = entry.get_definition().trim();

            // exclude APPLICATION_CONTEXT and APPLICATION_PROTOCOL_DEFINITION
            if definition.starts_with("APPLICATION_CONTEXT")
                || definition.starts_with("APPLICATION_PROTOCOL_DEFINITION")
            {
                continue;
            }

            // create new entry and update the references
            let mut new_entry = entry.clone();
            new_entry.update_references(update_id);

            // catch special case of MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION
            if definition.starts_with("MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION") {
                Self::get_ids_from_mechanical_part(&new_entry, &mut self.mechanical_design_ids);
                continue;
            } else {
                self.step_data.add_entry(new_entry);
            }

            max_id = max_id.max(entry.get_id());
        }

        // update the id counter to the new max id
        self.id_counter = max_id;

        Ok(())
    }

    /// Writes the final MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION entry.
    fn write_mechanical_part_entries(&mut self) {
        // write related entries
        let length = self.add_entry("(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT(.MILLI.,.METRE.))");
        let angle_units = self.add_entry("(NAMED_UNIT(*)PLANE_ANGLE_UNIT()SI_UNIT($,.RADIAN.))");
        self.add_entry(&format!(
            "PLANE_ANGLE_MEASURE_WITH_UNIT(PLANE_ANGLE_MEASURE(1.745329251994E-02),#{})",
            angle_units
        ));
        let dim_exp = self.add_entry("DIMENSIONAL_EXPONENTS(0.,0.,0.,0.,0.,0.,0.)");
        let angle = self.add_entry(&format!(
            "(CONVERSION_BASED_UNIT('DEGREE',#{})NAMED_UNIT(#101)PLANE_ANGLE_UNIT())",
            dim_exp
        ));
        let unit = self.add_entry("(NAMED_UNIT(*)SI_UNIT($,.STERADIAN.)SOLID_ANGLE_UNIT())");
        let uncertain = self.add_entry(&format!("UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(10.E-03),#{},'distance_accuracy_value','Confusion accuracy')", length));
        let full = self.add_entry(&format!("(GEOMETRIC_REPRESENTATION_CONTEXT(3)GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#{}))GLOBAL_UNIT_ASSIGNED_CONTEXT((#{},#{},#{}))REPRESENTATION_CONTEXT('',''))", uncertain, length, angle, unit));

        // compile list of referenced ids
        let mut list = String::new();
        for id in self.mechanical_design_ids.iter() {
            if list.is_empty() {
                list.push_str(&format!("#{}", id));
            } else {
                list.push_str(&format!(",#{}", id));
            }
        }

        let entry = format!(
            "MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION('',({}),#{})",
            list, full
        );

        self.add_entry(&entry);
    }

    /// Creates a new node in the step data. Returns a tuple consisting of the PRODUCT_DEFINITION
    /// id and the SHAPE_REPRESENTATION id.
    ///
    /// # Arguments
    /// * `node` - The node to be created.
    fn create_node(&mut self, node: &Node) -> NodeStepIds {
        let label = node.get_label();

        let start_id = self.add_entry("CARTESIAN_POINT('',(0.,0.,0.))");
        self.add_entry("DIRECTION('',(0.,0.,1.))");
        self.add_entry("DIRECTION('',(1.,0.,0.))");
        self.add_entry(&format!(
            "AXIS2_PLACEMENT_3D('',#{},#{},#{})",
            start_id,
            start_id + 1,
            start_id + 2
        ));

        self.add_entry("(NAMED_UNIT(*)SI_UNIT($,.STERADIAN.)SOLID_ANGLE_UNIT())");
        self.add_entry("(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT(.MILLI.,.METRE.))");
        self.add_entry("(NAMED_UNIT(*)PLANE_ANGLE_UNIT()SI_UNIT($,.RADIAN.))");

        self.add_entry("PRODUCT_CONTEXT('',#1,'mechanical')");
        self.add_entry(&format!(
            "PRODUCT('{}','{}','',(#{}))",
            label,
            label,
            start_id + 7
        ));
        self.add_entry("PRODUCT_DEFINITION_CONTEXT('part_definition',#1,'')");
        self.add_entry(&format!(
            "PRODUCT_DEFINITION_FORMATION('','',#{})",
            start_id + 8
        ));
        let product_definition_id = self.add_entry(&format!(
            "PRODUCT_DEFINITION('','',#{},#{})",
            start_id + 10,
            start_id + 9
        ));
        self.add_entry(&format!(
            "PRODUCT_DEFINITION_SHAPE('',$,#{})",
            start_id + 11
        ));
        self.add_entry(&format!(
            "PRODUCT_RELATED_PRODUCT_CATEGORY('component','',(#{}))",
            start_id + 8
        ));
        self.add_entry(&format!("UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(0.1E-12),#{},'distance accuracy value','edge curve and vertex point accuracy')", start_id + 5));
        self.add_entry(&format!("(GEOMETRIC_REPRESENTATION_CONTEXT(3)GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#{}))GLOBAL_UNIT_ASSIGNED_CONTEXT((#{},#{},#{}))REPRESENTATION_CONTEXT('',''))", start_id + 14, start_id + 5, start_id + 6, start_id + 4));
        let shape_representation_id = self.add_entry(&format!(
            "SHAPE_REPRESENTATION('{}',(#{}),#{})",
            label,
            start_id + 3,
            start_id + 15
        ));
        self.add_entry(&format!(
            "SHAPE_DEFINITION_REPRESENTATION(#{},#{})",
            start_id + 12,
            start_id + 16
        ));

        NodeStepIds {
            product_definition_id,
            shape_representation_id,
        }
    }

    /// Creates a parent-child relation between the given parent and child node.
    ///
    /// # Arguments
    /// * `parent_label` - The label of the parent node.
    /// * `child_label` - The label of the child node.
    /// * `parent_ids` - The step ids of the parent node.
    /// * `child_ids` - The step ids of the child node.
    fn create_parent_child_relation(
        &mut self,
        parent_label: &str,
        child_label: &str,
        parent_ids: NodeStepIds,
        child_ids: NodeStepIds,
    ) {
        let start_id = self.add_entry("CARTESIAN_POINT('',(0.,0.,0.))");
        self.add_entry("DIRECTION('',(0.,0.,1.))");
        self.add_entry("DIRECTION('',(1.,0.,0.))");
        self.add_entry(&format!(
            "AXIS2_PLACEMENT_3D('',#{},#{},#{})",
            start_id,
            start_id + 1,
            start_id + 2
        ));
        self.add_entry(&format!(
            "ITEM_DEFINED_TRANSFORMATION('','',#{},#{})",
            start_id + 3,
            start_id + 3
        ));
        self.add_entry(&format!(
            "(REPRESENTATION_RELATIONSHIP('Child > Parent','{} > {}',#{}, #{})REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION(#{})SHAPE_REPRESENTATION_RELATIONSHIP())",
            child_label,
            parent_label,
            parent_ids.shape_representation_id,
            child_ids.shape_representation_id,
            start_id + 4
        ));
        self.add_entry(&format!(
            "NEXT_ASSEMBLY_USAGE_OCCURRENCE('{}','','{}',#{},#{},'{}')",
            child_label,
            child_label,
            parent_ids.product_definition_id,
            child_ids.product_definition_id,
            child_label,
        ));

        self.add_entry(&format!(
            "PRODUCT_DEFINITION_SHAPE('{}',$,#{})",
            child_label,
            start_id + 6
        ));

        self.add_entry(&format!(
            "CONTEXT_DEPENDENT_SHAPE_REPRESENTATION(#{},#{})",
            start_id + 5,
            start_id + 7
        ));
    }

    /// Extracts all ids for the items defined in MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION entry.
    ///
    /// # Arguments
    /// * `entry` - The MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION entry.
    /// * `ids` - The list of ids where the extracted ids are added.
    fn get_ids_from_mechanical_part(entry: &StepEntry, ids: &mut Vec<u64>) {
        let mut entry_ids = entry.get_references();
        if entry_ids.is_empty() {
            error!("No references found in MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION entry");
            return;
        } else {
            entry_ids.pop();
        }

        debug!(
            "MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION got id {} and has {} ids",
            entry.get_id(),
            entry_ids.len()
        );

        ids.append(&mut entry_ids);
    }
}

/// The ids being generated for a node while creating the step data.
#[derive(Debug, Clone, Copy)]
struct NodeStepIds {
    pub product_definition_id: u64,
    pub shape_representation_id: u64,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_mechanical_design_entry() {
        let s = "MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION('',(#24),#187);";
        let entry = StepEntry::new(1, s);
        let mut ids = Vec::new();

        StepMerger::get_ids_from_mechanical_part(&entry, &mut ids);

        assert_eq!(ids, vec![24]);
    }
}
