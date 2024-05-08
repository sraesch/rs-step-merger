use crate::{Assembly, Node, Result};

use log::info;

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

        //
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
}

/// The ids being generated for a node while creating the step data.
#[derive(Debug, Clone, Copy)]
struct NodeStepIds {
    pub product_definition_id: u64,
    pub shape_representation_id: u64,
}
