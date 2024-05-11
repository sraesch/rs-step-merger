use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use crate::{identity_matrix, Assembly, Error, Node, Result};

use log::{debug, error, info};

use super::{StepData, StepEntry};

/// The function consumes the given assembly structure and creates step data consisting of the
/// given assembly structure
///
/// # Arguments
/// * `assembly` - The assembly structure to be merged.
/// * `load_references` - Flag to indicate if external references should be loaded.
pub fn merge_assembly_structure_to_step(
    assembly: &Assembly,
    load_references: bool,
) -> Result<StepData> {
    let mut merger = StepMerger::new(assembly);
    merger.merge(load_references)?;

    Ok(merger.step_data)
}

/// The internal step merge operator
struct StepMerger<'a> {
    /// The assembly structure to be merged.
    assembly: &'a Assembly,

    /// The id for the step entries.
    default_coordinate_system: u64,

    /// The id counter for the step entries.
    id_counter: u64,

    // The serialized step data
    step_data: StepData,

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
            default_coordinate_system: 0,
            id_counter: 0,
            step_data,
            mechanical_design_ids: Vec::new(),
        }
    }

    /// Merges the assembly structure into a single monolithic step file.
    ///
    /// # Arguments
    /// * `load_references` - Flag to indicate if external references should be loaded.
    pub fn merge(&mut self, load_references: bool) -> Result<()> {
        info!("Merging assembly structure into step file...");
        self.create_app_context();

        // create default coordinate system
        let coord_id = self.add_entry("CARTESIAN_POINT('',(0.,0.,0.))");
        self.add_entry("DIRECTION('',(0.,0.,1.))");
        self.add_entry("DIRECTION('',(1.,0.,0.))");
        self.default_coordinate_system = self.add_entry(&format!(
            "AXIS2_PLACEMENT_3D('',#{},#{},#{})",
            coord_id,
            coord_id + 1,
            coord_id + 2
        ));

        // create the nodes of the assembly structure and collect the node product definition and
        // shape representation ids
        let node_step_ids: Vec<NodeStepIds> = self
            .assembly
            .nodes
            .iter()
            .map(|node| self.create_node(node))
            .collect();

        // create the parent-child relations between the assembly nodes
        for (node, node_ids) in self.assembly.nodes.iter().zip(node_step_ids.iter()) {
            for child in node.get_children() {
                let child_ids = &node_step_ids[*child];
                let child = &self.assembly.nodes[*child];
                self.create_parent_child_relation(
                    node.get_label(),
                    child.get_label(),
                    *node_ids,
                    *child_ids,
                    child.get_transform(),
                );
            }
        }

        // load all referenced step files and add them to the current step data
        if load_references {
            let mut reference_map: HashMap<String, Vec<NodeStepIds>> = HashMap::new();
            for node in self.assembly.nodes.iter() {
                if let Some(link) = node.get_link() {
                    if !reference_map.contains_key(link) {
                        let root_nodes = self.load_and_add_step(link)?;
                        reference_map.insert(link.to_owned(), root_nodes);
                    }
                }
            }

            // create the parent-child relations between the assembly nodes and the referenced step
            // files
            for (node, node_ids) in self.assembly.nodes.iter().zip(node_step_ids.iter()) {
                if let Some(link) = node.get_link() {
                    let root_nodes = reference_map.get(link).unwrap();
                    for child_ids in root_nodes.iter() {
                        self.create_parent_child_relation(
                            node.get_label(),
                            node.get_label(),
                            *node_ids,
                            *child_ids,
                            &identity_matrix(),
                        );
                    }
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
    /// Returns the STEP ids of the root nodes.
    ///
    /// # Arguments
    /// * `file_path` - The path to the step file to be loaded.
    fn load_and_add_step<P: AsRef<Path>>(&mut self, file_path: P) -> Result<Vec<NodeStepIds>> {
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
        let start_entry_index = self.step_data.get_entries().len();
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
            max_id = max_id.max(new_entry.get_id());

            // catch special case of MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION
            if definition.starts_with("MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION") {
                Self::get_ids_from_mechanical_part(&new_entry, &mut self.mechanical_design_ids);
                continue;
            } else {
                self.step_data.add_entry(new_entry);
            }
        }

        // update the id counter to the new max id
        self.id_counter = max_id;

        // extract the root nodes from the loaded step data
        let new_entries = &self.step_data.get_entries()[start_entry_index..];
        let root_nodes = Self::find_root_nodes(new_entries.iter());

        Ok(root_nodes)
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

        // add metadata
        for metadata in node.get_metadata() {
            let prop_def_id = self.add_entry(&format!(
                "PROPERTY_DEFINITION('{}','',#{})",
                metadata.key, product_definition_id
            ));
            let desc_rep_item_id = self.add_entry(&format!(
                "DESCRIPTIVE_REPRESENTATION_ITEM('{}','{}')",
                metadata.key, metadata.value
            ));

            let rep_id = self.add_entry(&format!("REPRESENTATION('',(#{}),$)", desc_rep_item_id));

            self.add_entry(&format!(
                "PROPERTY_DEFINITION_REPRESENTATION(#{},#{})",
                prop_def_id, rep_id
            ));
        }

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
    /// * `transform` - The transformation matrix from the parent to the child node.
    fn create_parent_child_relation(
        &mut self,
        parent_label: &str,
        child_label: &str,
        parent_ids: NodeStepIds,
        child_ids: NodeStepIds,
        transform: &[f32; 16],
    ) {
        // determine the position and translate it from meter to millimeter
        let position = [
            transform[12] * 1000.0,
            transform[13] * 1000.0,
            transform[14] * 1000.0,
        ];

        // extract the position, x-axis and z-axis vector
        let x_axis = &transform[0..3];
        let z_axis = &transform[8..11];

        let start_id = self.add_entry(&format!(
            "CARTESIAN_POINT('',({},{},{}))",
            position[0], position[1], position[2],
        ));
        self.add_entry(&format!(
            "DIRECTION('',({},{},{}))",
            z_axis[0], z_axis[1], z_axis[2]
        ));
        self.add_entry(&format!(
            "DIRECTION('',({},{},{}))",
            x_axis[0], x_axis[1], x_axis[2]
        ));
        self.add_entry(&format!(
            "AXIS2_PLACEMENT_3D('',#{},#{},#{})",
            start_id,
            start_id + 1,
            start_id + 2
        ));
        self.add_entry(&format!(
            "ITEM_DEFINED_TRANSFORMATION('','',#{},#{})",
            self.default_coordinate_system,
            start_id + 3
        ));
        self.add_entry(&format!(
            "(REPRESENTATION_RELATIONSHIP('Child > Parent','{} > {}',#{}, #{})REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION(#{})SHAPE_REPRESENTATION_RELATIONSHIP())",
            child_label,
            parent_label,
            child_ids.shape_representation_id,
            parent_ids.shape_representation_id,
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

    /// Finds the root nodes in the given entries.
    ///
    /// # Arguments
    /// * `entries` - The entries to find the root nodes in.
    fn find_root_nodes<'b, I>(entries: I) -> Vec<NodeStepIds>
    where
        I: Iterator<Item = &'b StepEntry>,
    {
        // We have the following entities:
        // * PRODUCT_DEFINITION
        // * SHAPE_REPRESENTATION
        // * PRODUCT_DEFINITION_SHAPE
        // * SHAPE_DEFINITION_REPRESENTATION
        // * NEXT_ASSEMBLY_USAGE_OCCURRENCE
        //
        // The references between the entities are as follows:
        // * NEXT_ASSEMBLY_USAGE_OCCURRENCE -> PRODUCT_DEFINITION
        // * PRODUCT_DEFINITION_SHAPE -> PRODUCT_DEFINITION
        // * SHAPE_DEFINITION_REPRESENTATION -> SHAPE_REPRESENTATION
        // * SHAPE_DEFINITION_REPRESENTATION -> PRODUCT_DEFINITION_SHAPE
        //
        // We are interested to find the root node which is the node that has no parent, i.e. the
        // product definition where no NEXT_ASSEMBLY_USAGE_OCCURRENCE references it.
        // We then have to return the SHAPE_REPRESENTATION and PRODUCT_DEFINITION_SHAPE ids.
        let mut shape_def_rep_to_shape_rep: HashMap<u64, u64> = HashMap::new();
        let mut prod_def_shape_to_shape_def_rep: HashMap<u64, u64> = HashMap::new();
        let mut prod_def_to_prod_def_shape: Vec<(u64, u64)> = Vec::new();
        let mut prod_def_assembly_occurrences: HashSet<u64> = HashSet::new();

        for entry in entries {
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
                        continue;
                    }

                    let prod_def_shape_id = references[0];
                    let shape_rep_id = references[1];

                    shape_def_rep_to_shape_rep.insert(shape_def_rep_id, shape_rep_id);
                    prod_def_shape_to_shape_def_rep.insert(prod_def_shape_id, shape_def_rep_id);
                }
                "PRODUCT_DEFINITION_SHAPE" => {
                    let prod_def_shape_id = entry.get_id();
                    let references = entry.get_references();
                    if references.is_empty() {
                        error!(
                            "PRODUCT_DEFINITION_SHAPE entry with id {} has no references",
                            prod_def_shape_id,
                        );
                        continue;
                    }

                    let prod_def_id = references.last().unwrap();
                    prod_def_to_prod_def_shape.push((*prod_def_id, prod_def_shape_id));
                }
                "NEXT_ASSEMBLY_USAGE_OCCURRENCE" => {
                    let references = entry.get_references();
                    if references.len() != 2 {
                        error!(
                            "NEXT_ASSEMBLY_USAGE_OCCURRENCE entry with id {} has {} references",
                            entry.get_id(),
                            references.len()
                        );
                        continue;
                    }

                    let prod_def_id = references[1];
                    prod_def_assembly_occurrences.insert(prod_def_id);
                }
                _ => {}
            }
        }

        // find all root nodes
        let mut result = Vec::new();
        for (prod_def_id, prod_def_shape_id) in prod_def_to_prod_def_shape.iter() {
            // skip if the product definition is referenced by an assembly occurrence
            if prod_def_assembly_occurrences.contains(prod_def_id) {
                continue;
            }

            // try to find the shape definition representation
            if let Some(shape_def_rep_id) = prod_def_shape_to_shape_def_rep.get(&prod_def_shape_id)
            {
                // try to find the shape representation
                if let Some(shape_rep_id) = shape_def_rep_to_shape_rep.get(shape_def_rep_id) {
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

/// The ids being generated for a node while creating the step data.
#[derive(Debug, Clone, Copy)]
struct NodeStepIds {
    pub product_definition_id: u64,
    pub shape_representation_id: u64,
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn test_parse_mechanical_design_entry() {
        let s = "MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION('',(#24),#187);";
        let entry = StepEntry::new(1, s);
        let mut ids = Vec::new();

        StepMerger::get_ids_from_mechanical_part(&entry, &mut ids);

        assert_eq!(ids, vec![24]);
    }

    #[test]
    fn test_extract_keyword() {
        let s = "PRODUCT_DEFINITION_SHAPE('',#,#);";
        let keyword = StepMerger::extract_keyword(s);

        assert_eq!(keyword, "PRODUCT_DEFINITION_SHAPE");

        let s = "  FOOBAR_BLUB( );  ";
        let keyword = StepMerger::extract_keyword(s);

        assert_eq!(keyword, "FOOBAR_BLUB");
    }

    #[test]
    fn test_find_root_nodes1() {
        let source = include_str!("../../../test_data/minimal-structure.stp");
        let step_data = StepData::from_str(source).unwrap();

        let entries = step_data.get_entries();
        let root_nodes = StepMerger::find_root_nodes(entries.iter());

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
        let root_nodes = StepMerger::find_root_nodes(entries.iter());

        assert_eq!(root_nodes.len(), 1);

        assert_eq!(root_nodes[0].product_definition_id, 14);
        assert_eq!(root_nodes[0].shape_representation_id, 31);
    }
}
