use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Write},
};

use log::{debug, error, info, trace};

use crate::{
    identity_matrix,
    merge::{
        root_nodes::FindRootNodes,
        utils::{get_ids_from_mechanical_part, NodeStepIds},
    },
    step::{STEPReader, StepEntry, StepWriter},
    Assembly, Error, Node, Result,
};

use self::buffered_iterator::BufferedIterator;

mod buffered_iterator;
mod root_nodes;
mod utils;

/// The function resolves the given file path and returns the file handle.
/// Helper function for resolving relative file paths.
#[inline]
pub fn resolve_file(file_path: &str) -> Result<File> {
    let file = File::open(file_path)?;

    Ok(file)
}

/// The function consumes the given assembly structure and writes the merged step data to the given
/// writer.
/// All references to external step files are loaded and merged into the final step data.
/// If a reference cannot be loaded, an error is dumped to the log and the process continues.
/// The whole merging process is executed in a streaming fashion to reduce the memory footprint.
///
/// # Arguments
/// * `assembly` - The assembly structure to merged.
/// * `load_references` - Flag to indicate if external references should be loaded.
/// * `writer` - The writer for the merged step file.
pub fn merge_assembly_structure_to_step<W>(
    assembly: &Assembly,
    load_references: bool,
    writer: W,
) -> Result<()>
where
    W: Write,
{
    let resolver = resolve_file;
    merge_assembly_structure_to_step_with_resolver(assembly, load_references, writer, resolver)
}

/// The function consumes the given assembly structure and writes the merged step data to the given
/// writer.
/// All references to external step files are loaded and merged into the final step data using the
/// given reference resolver.
/// If a reference cannot be resolved, an error is dumped to the log and the process continues.
/// The whole merging process is executed in a streaming fashion to reduce the memory footprint.
///
/// # Arguments
/// * `assembly` - The assembly structure to merged.
/// * `load_references` - Flag to indicate if external references should be loaded.
/// * `writer` - The writer for the merged step file.
pub fn merge_assembly_structure_to_step_with_resolver<W, R, Resolver>(
    assembly: &Assembly,
    load_references: bool,
    writer: W,
    resolver: Resolver,
) -> Result<()>
where
    W: Write,
    R: Read,
    Resolver: FnMut(&str) -> Result<R>,
{
    let mut merger = StepMerger::new(writer, assembly, resolver)?;
    merger.merge(load_references)?;

    Ok(())
}

/// The internal step merge operator
struct StepMerger<'a, W, R, Resolver>
where
    W: Write,
    R: Read,
    Resolver: FnMut(&str) -> Result<R>,
{
    /// The assembly structure to be merged.
    assembly: &'a Assembly,

    /// The writer for the merged step file.
    writer: StepWriter<W>,

    /// The reference resolver to load external step files.
    resolver: Resolver,

    /// The id of the STEP entry for the default coordinate system.
    default_coordinate_system: u64,

    /// The id counter for the step entries.
    id_counter: u64,

    /// The list of referenced mechanical design entries
    mechanical_design_ids: Vec<u64>,
}

impl<'a, W: Write, R: Read, Resolver: FnMut(&str) -> Result<R>> StepMerger<'a, W, R, Resolver> {
    /// Creates a new step merger with the given writer and assembly structure.
    ///
    /// # Arguments
    /// * `writer` - The writer for the merged step file.
    /// * `assembly` - The assembly structure to be merged.
    pub fn new(writer: W, assembly: &'a Assembly, resolver: Resolver) -> Result<Self> {
        let protocol = vec!["AP203_CONFIGURATION_CONTROLLED_3D_DESIGN_OF_MECHANICAL_PARTS_AND_ASSEMBLIES_MIM_LF { 1 0 10303 403 1 1 4 }".to_owned()];
        let step_writer = StepWriter::new(writer, "2;1", "", &protocol)?;

        Ok(StepMerger {
            assembly,
            writer: step_writer,
            resolver,
            default_coordinate_system: 0,
            id_counter: 0,
            mechanical_design_ids: Vec::new(),
        })
    }

    /// Merges the assembly structure into a single monolithic step file.
    ///
    /// # Arguments
    /// * `load_references` - Flag to indicate if external references should be loaded.
    pub fn merge(&mut self, load_references: bool) -> Result<()> {
        info!("Merging assembly structure into step file...");
        self.create_app_context()?;

        // create default coordinate system
        trace!("Create default coordinate system...");
        let coord_id = self.add_entry("CARTESIAN_POINT('',(0.,0.,0.))")?;
        self.add_entry("DIRECTION('',(0.,0.,1.))")?;
        self.add_entry("DIRECTION('',(1.,0.,0.))")?;
        self.default_coordinate_system = self.add_entry(&format!(
            "AXIS2_PLACEMENT_3D('',#{},#{},#{})",
            coord_id,
            coord_id + 1,
            coord_id + 2
        ))?;
        trace!(
            "Create default coordinate system...DONE, ID={}",
            self.default_coordinate_system
        );

        // create the nodes of the assembly structure and collect the node product definition and
        // shape representation ids
        info!("Create assembly nodes...");
        let mut node_step_ids: Vec<NodeStepIds> = Vec::with_capacity(self.assembly.nodes.len());
        for node in self.assembly.nodes.iter() {
            trace!("Create node {}...", node.get_label());
            let node_ids = self.create_node(node)?;
            trace!(
                "Create node {}...DONE with PRODUCT_DEFINITION={}, SHAPE_REPRESENTATION={}",
                node.get_label(),
                node_ids.product_definition_id,
                node_ids.shape_representation_id
            );

            node_step_ids.push(node_ids);
        }
        info!(
            "Create assembly nodes...DONE, {} nodes created",
            node_step_ids.len()
        );

        // create the parent-child relations between the assembly nodes
        info!("Create parent-child relations...");
        for (node, node_ids) in self.assembly.nodes.iter().zip(node_step_ids.iter()) {
            for child in node.get_children() {
                let child_ids = &node_step_ids[*child];
                let child = &self.assembly.nodes[*child];

                trace!(
                    "Create parent-child relation between {} and {}...",
                    node.get_label(),
                    child.get_label()
                );
                self.create_parent_child_relation(
                    node.get_label(),
                    child.get_label(),
                    *node_ids,
                    *child_ids,
                    child.get_transform(),
                )?;
            }
        }

        info!("Create parent-child relations...DONE");

        // load all referenced step files and add them to the current step data
        if load_references {
            info!("Load and add referenced step files...");
            let mut reference_map: HashMap<String, Vec<NodeStepIds>> = HashMap::new();
            for node in self.assembly.nodes.iter() {
                trace!("Check node {} for references...", node.get_label());
                if let Some(link) = node.get_link() {
                    info!("Got link {}...", link);
                    if !reference_map.contains_key(link) {
                        debug!("Load and add step file {}...", link);
                        match self.load_and_add_step(link) {
                            Ok(root_nodes) => {
                                debug!("Root nodes: {:?}...", root_nodes);
                                reference_map.insert(link.to_owned(), root_nodes);
                            }
                            Err(err) => {
                                error!("Error loading step file {}: {}", link, err);
                                continue;
                            }
                        }
                    } else {
                        trace!("Link {} already loaded...", link);
                    }
                }
            }

            // create the parent-child relations between the assembly nodes and the referenced step
            // files
            info!("Create parent-child relations for referenced step files...");
            for (node, node_ids) in self.assembly.nodes.iter().zip(node_step_ids.iter()) {
                if let Some(link) = node.get_link() {
                    if let Some(root_nodes) = reference_map.get(link) {
                        for child_ids in root_nodes.iter() {
                            self.create_parent_child_relation(
                                node.get_label(),
                                node.get_label(),
                                *node_ids,
                                *child_ids,
                                &identity_matrix(),
                            )?;
                        }
                    }
                }
            }
            info!("Create parent-child relations for referenced step files...DONE");
        }

        debug!("Write mechanical part entries...");
        self.write_mechanical_part_entries()?;
        debug!("Write mechanical part entries...DONE");

        info!("Finalize step file...");
        self.writer.finalize()?;
        info!("Finalize step file...DONE");

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
    fn add_entry(&mut self, definition: &str) -> Result<u64> {
        let id = self.get_new_id();
        let entry = StepEntry::new(id, definition);

        self.add_entry_full(&entry)?;

        Ok(id)
    }

    /// Adds a new entry to the step data.
    ///
    /// # Arguments
    /// * `entry` - The entry to be added.
    #[inline]
    fn add_entry_full(&mut self, entry: &StepEntry) -> Result<()> {
        self.writer.write_entry(entry)?;

        Ok(())
    }

    /// Creates the application context and protocol definition.
    fn create_app_context(&mut self) -> Result<()> {
        let app_id = self.add_entry(
            "APPLICATION_CONTEXT('Configuration controlled 3D designs of mechanical parts and assemblies')",
        )?;

        assert_eq!(app_id, 1);

        self.add_entry("APPLICATION_PROTOCOL_DEFINITION('international standard', 'configuration_control_3d_design_ed2_mim',2004, #1)",
        )?;

        Ok(())
    }

    /// Loads the given step file and adds the loaded step data to the current step data.
    /// Returns the STEP ids of the root nodes.
    ///
    /// # Arguments
    /// * `link` - The link to the step file.
    fn load_and_add_step(&mut self, link: &str) -> Result<Vec<NodeStepIds>> {
        info!("Load step file {}...", link);

        trace!("Open step file {}...", link);
        let r = (self.resolver)(link)?;

        trace!("Create step reader...");
        let parser = STEPReader::new(r)?;

        debug!("Stream step entries...");
        let result = self.load_and_add_step_entries(parser.into_iter(), link)?;
        debug!("Stream step entries...DONE");

        info!("Load step file {}...DONE", link);
        Ok(result)
    }

    /// Adds the given step file as step entries to the current step data.
    /// Returns the STEP ids of the root nodes.
    ///
    /// # Arguments
    /// * `entries` - The entries of another step file to be added.
    /// * `filename` - The name of the step file.
    fn load_and_add_step_entries<I>(
        &mut self,
        entries: I,
        filename: &str,
    ) -> Result<Vec<NodeStepIds>>
    where
        I: Iterator<Item = Result<StepEntry>>,
    {
        let mut entries = BufferedIterator::new(entries);

        // Find the id for the APPLICATION_CONTEXT entry. We use the buffered iterator to reuse the
        // entries that have been read to find the APPLICATION_CONTEXT entry.
        debug!(
            "Find APPLICATION_CONTEXT entry in step file {}...",
            filename
        );
        entries.set_buffering_mode();
        let mut app_context_id = 0;
        for (index, entry) in entries.iter().enumerate() {
            let entry = entry?;
            if entry
                .get_definition()
                .trim_start()
                .starts_with("APPLICATION_CONTEXT")
            {
                app_context_id = entry.get_id();
                debug!(
                    "APPLICATION_CONTEXT entry is {} at index={}",
                    app_context_id, index
                );
                break;
            }
        }

        if app_context_id == 0 {
            return Err(Error::AppContextMissing(filename.to_string()));
        }

        entries.reset();

        // We define an update function to make sure that:
        // - all ids are shifted by the current id counter (offset)
        // - the APPLICATION_CONTEXT id is redirected to 1
        let id_offset = self.id_counter;
        debug!("ID offset is {}", id_offset);
        let update_id = |id: u64| {
            if id == app_context_id {
                1
            } else {
                id + id_offset
            }
        };

        // stream the entries into the output step file
        let mut max_id = 0u64;
        let mut find_root_nodes = FindRootNodes::new();
        for entry in entries.iter() {
            let entry = entry?;
            let definition = entry.get_definition().trim_start();

            // exclude APPLICATION_CONTEXT and APPLICATION_PROTOCOL_DEFINITION
            if definition.starts_with("APPLICATION_CONTEXT")
                || definition.starts_with("APPLICATION_PROTOCOL_DEFINITION")
            {
                continue;
            }

            // create new updated entry where the ids have been patched
            let new_entry = entry.update_references(update_id);
            max_id = max_id.max(new_entry.get_id());

            // catch special case of MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION
            if definition.starts_with("MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION") {
                get_ids_from_mechanical_part(&new_entry, &mut self.mechanical_design_ids);
                continue;
            } else {
                find_root_nodes.add_entry(&new_entry);
                self.add_entry_full(&new_entry)?;
            }
        }

        // update the id counter to the new max id
        self.id_counter = max_id;

        // extract the root nodes from the loaded step data
        let root_nodes = find_root_nodes.get_root_nodes();
        if root_nodes.is_empty() {
            error!("No root nodes found in step file {}", filename);
        }

        Ok(root_nodes)
    }

    /// Writes the final MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION entry.
    fn write_mechanical_part_entries(&mut self) -> Result<()> {
        // write related entries
        let length = self.add_entry("(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT(.MILLI.,.METRE.))")?;
        let angle_units = self.add_entry("(NAMED_UNIT(*)PLANE_ANGLE_UNIT()SI_UNIT($,.RADIAN.))")?;
        self.add_entry(&format!(
            "PLANE_ANGLE_MEASURE_WITH_UNIT(PLANE_ANGLE_MEASURE(1.745329251994E-02),#{})",
            angle_units
        ))?;
        let dim_exp = self.add_entry("DIMENSIONAL_EXPONENTS(0.,0.,0.,0.,0.,0.,0.)")?;
        let angle = self.add_entry(&format!(
            "(CONVERSION_BASED_UNIT('DEGREE',#{})NAMED_UNIT(#101)PLANE_ANGLE_UNIT())",
            dim_exp
        ))?;
        let unit = self.add_entry("(NAMED_UNIT(*)SI_UNIT($,.STERADIAN.)SOLID_ANGLE_UNIT())")?;
        let uncertain = self.add_entry(&format!("UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(10.E-03),#{},'distance_accuracy_value','Confusion accuracy')", length))?;
        let full = self.add_entry(&format!("(GEOMETRIC_REPRESENTATION_CONTEXT(3)GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#{}))GLOBAL_UNIT_ASSIGNED_CONTEXT((#{},#{},#{}))REPRESENTATION_CONTEXT('',''))", uncertain, length, angle, unit))?;

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

        self.add_entry(&entry)?;

        Ok(())
    }

    /// Creates a new node in the step data. Returns a tuple consisting of the PRODUCT_DEFINITION
    /// id and the SHAPE_REPRESENTATION id.
    ///
    /// # Arguments
    /// * `node` - The node to be created.
    fn create_node(&mut self, node: &Node) -> Result<NodeStepIds> {
        let label = node.get_label();

        let start_id = self.add_entry("CARTESIAN_POINT('',(0.,0.,0.))")?;
        self.add_entry("DIRECTION('',(0.,0.,1.))")?;
        self.add_entry("DIRECTION('',(1.,0.,0.))")?;
        self.add_entry(&format!(
            "AXIS2_PLACEMENT_3D('',#{},#{},#{})",
            start_id,
            start_id + 1,
            start_id + 2
        ))?;

        self.add_entry("(NAMED_UNIT(*)SI_UNIT($,.STERADIAN.)SOLID_ANGLE_UNIT())")?;
        self.add_entry("(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT(.MILLI.,.METRE.))")?;
        self.add_entry("(NAMED_UNIT(*)PLANE_ANGLE_UNIT()SI_UNIT($,.RADIAN.))")?;

        self.add_entry("PRODUCT_CONTEXT('',#1,'mechanical')")?;
        self.add_entry(&format!(
            "PRODUCT('{}','{}','',(#{}))",
            label,
            label,
            start_id + 7
        ))?;
        self.add_entry("PRODUCT_DEFINITION_CONTEXT('part_definition',#1,'')")?;
        self.add_entry(&format!(
            "PRODUCT_DEFINITION_FORMATION('','',#{})",
            start_id + 8
        ))?;
        let product_definition_id = self.add_entry(&format!(
            "PRODUCT_DEFINITION('','',#{},#{})",
            start_id + 10,
            start_id + 9
        ))?;
        self.add_entry(&format!(
            "PRODUCT_DEFINITION_SHAPE('',$,#{})",
            start_id + 11
        ))?;
        self.add_entry(&format!(
            "PRODUCT_RELATED_PRODUCT_CATEGORY('component','',(#{}))",
            start_id + 8
        ))?;
        self.add_entry(&format!("UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(0.1E-12),#{},'distance accuracy value','edge curve and vertex point accuracy')", start_id + 5))?;
        self.add_entry(&format!("(GEOMETRIC_REPRESENTATION_CONTEXT(3)GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#{}))GLOBAL_UNIT_ASSIGNED_CONTEXT((#{},#{},#{}))REPRESENTATION_CONTEXT('',''))", start_id + 14, start_id + 5, start_id + 6, start_id + 4))?;
        let shape_representation_id = self.add_entry(&format!(
            "SHAPE_REPRESENTATION('{}',(#{}),#{})",
            label,
            start_id + 3,
            start_id + 15
        ))?;
        self.add_entry(&format!(
            "SHAPE_DEFINITION_REPRESENTATION(#{},#{})",
            start_id + 12,
            start_id + 16
        ))?;

        // add metadata
        for metadata in node.get_metadata() {
            let prop_def_id = self.add_entry(&format!(
                "PROPERTY_DEFINITION('{}','',#{})",
                metadata.key, product_definition_id
            ))?;
            let desc_rep_item_id = self.add_entry(&format!(
                "DESCRIPTIVE_REPRESENTATION_ITEM('{}','{}')",
                metadata.key, metadata.value
            ))?;

            let rep_id =
                self.add_entry(&format!("REPRESENTATION('',(#{}),$)", desc_rep_item_id))?;

            self.add_entry(&format!(
                "PROPERTY_DEFINITION_REPRESENTATION(#{},#{})",
                prop_def_id, rep_id
            ))?;
        }

        Ok(NodeStepIds {
            product_definition_id,
            shape_representation_id,
        })
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
    ) -> Result<()> {
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
        ))?;
        self.add_entry(&format!(
            "DIRECTION('',({},{},{}))",
            z_axis[0], z_axis[1], z_axis[2]
        ))?;
        self.add_entry(&format!(
            "DIRECTION('',({},{},{}))",
            x_axis[0], x_axis[1], x_axis[2]
        ))?;
        self.add_entry(&format!(
            "AXIS2_PLACEMENT_3D('',#{},#{},#{})",
            start_id,
            start_id + 1,
            start_id + 2
        ))?;
        self.add_entry(&format!(
            "ITEM_DEFINED_TRANSFORMATION('','',#{},#{})",
            self.default_coordinate_system,
            start_id + 3
        ))?;
        self.add_entry(&format!(
            "(REPRESENTATION_RELATIONSHIP('Child > Parent','{} > {}',#{}, #{})REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION(#{})SHAPE_REPRESENTATION_RELATIONSHIP())",
            child_label,
            parent_label,
            child_ids.shape_representation_id,
            parent_ids.shape_representation_id,
            start_id + 4
        ))?;
        self.add_entry(&format!(
            "NEXT_ASSEMBLY_USAGE_OCCURRENCE('{}','','{}',#{},#{},'{}')",
            child_label,
            child_label,
            parent_ids.product_definition_id,
            child_ids.product_definition_id,
            child_label,
        ))?;

        self.add_entry(&format!(
            "PRODUCT_DEFINITION_SHAPE('{}',$,#{})",
            child_label,
            start_id + 6
        ))?;

        self.add_entry(&format!(
            "CONTEXT_DEPENDENT_SHAPE_REPRESENTATION(#{},#{})",
            start_id + 5,
            start_id + 7
        ))?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::{
        io::{BufRead, BufReader, Cursor},
        sync::Arc,
    };

    use super::*;

    #[test]
    fn test_merge_assembly_structure_to_step_with_resolver() {
        let cube_stp = include_bytes!("../../../test_data/cube.stp");
        let sphere_stp = include_bytes!("../../../test_data/sphere.stp");
        let assembly = include_bytes!("../../../test_data/cube-and-sphere.json");
        let merged = include_bytes!("../../../test_data/cube-and-sphere.stp");

        let resolver = |link: &str| -> Result<_> {
            match link {
                "cube.stp" => Ok(Cursor::new(cube_stp.as_slice())),
                "sphere.stp" => Ok(Cursor::new(sphere_stp.as_slice())),
                _ => Err(Error::FailedOpenFile(
                    Arc::new(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        format!("File not found: {}", link),
                    )),
                    link.to_owned(),
                )),
            }
        };

        let mut output = Vec::new();
        merge_assembly_structure_to_step_with_resolver(
            &serde_json::from_slice::<Assembly>(assembly).unwrap(),
            true,
            &mut output,
            resolver,
        )
        .unwrap();

        let mut skip_lines = true;
        for (l, (expected, actual)) in BufReader::new(merged.as_ref())
            .lines()
            .zip(BufReader::new(output.as_slice()).lines())
            .enumerate()
        {
            let expected = expected.unwrap();
            let actual = actual.unwrap();

            if expected.starts_with("DATA;") {
                skip_lines = false;
            }

            if !skip_lines {
                assert_eq!(expected, actual, "Mismatch at line {}", l + 1);
            }
        }
    }
}
