use std::{path::Path, sync::Arc};

use crate::{Error, Result};
use serde::{Deserialize, Serialize};

/// Returns the identity matrix.
pub const fn identity_matrix() -> [f32; 16] {
    [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]
}

/// A single metadata entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataEntry {
    pub key: String,
    pub value: String,
}

/// A single node in the assembly tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    link: Option<String>,
    label: String,

    // Metadata entries associated with the node.
    #[serde(default)]
    metadata: Vec<MetadataEntry>,

    // Column-major encoded 4x4 transformation matrix. The default value is the identity matrix.
    #[serde(default = "identity_matrix")]
    transform: [f32; 16],

    #[serde(default)]
    children: Vec<usize>,
}

impl Node {
    /// Creates a new node with the given label.
    ///
    /// # Arguments
    /// * `label` - The label of the node.
    pub fn new(label: &str) -> Node {
        Node {
            link: None,
            label: label.to_owned(),
            metadata: Vec::new(),
            transform: identity_matrix(),
            children: Vec::new(),
        }
    }

    /// Adds a child to the node.
    ///
    /// # Arguments
    /// * `child` - The child node to be added.
    #[inline]
    pub fn add_child(&mut self, child: usize) {
        self.children.push(child);
    }

    /// Sets the link of the node.
    ///
    /// # Arguments
    /// * `link` - The link to be set.
    pub fn set_link(&mut self, link: &str) {
        self.link = Some(link.to_owned());
    }

    /// Returns the link of the node.
    pub fn get_link(&self) -> Option<&str> {
        self.link.as_deref()
    }

    /// Returns the label of the node.
    pub fn get_label(&self) -> &str {
        &self.label
    }

    /// Returns the children of the node.
    pub fn get_children(&self) -> &[usize] {
        &self.children
    }

    /// Returns the transformation matrix of the node.
    #[inline]
    pub fn get_transform(&self) -> &[f32; 16] {
        &self.transform
    }

    /// Returns the metadata
    #[inline]
    pub fn get_metadata(&self) -> &[MetadataEntry] {
        &self.metadata
    }
}

/// The assembly tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assembly {
    pub nodes: Vec<Node>,
}

impl Assembly {
    /// Creates a new assembly by loading the given file.
    ///
    /// # Arguments
    /// * `file` - The file to load the assembly from.
    pub fn from_file<P: AsRef<Path>>(file: P) -> Result<Assembly> {
        let filename_str = file.as_ref().to_string_lossy().to_string();
        let rdr = std::io::BufReader::new(
            std::fs::File::open(file)
                .map_err(|e| Error::FailedOpenFile(Arc::new(e), filename_str))?,
        );
        let assembly: Assembly =
            serde_json::from_reader(rdr).map_err(|e| Error::LoadAssembly(Arc::new(e)))?;

        assembly.is_valid()?;

        Ok(assembly)
    }

    /// Checks if the assembly is valid.
    pub fn is_valid(&self) -> Result<()> {
        let num_nodes = self.nodes.len();

        for node in self.nodes.iter() {
            for child in node.get_children() {
                if *child >= num_nodes {
                    return Err(Error::InvalidFormat(*child, node.get_label().to_string()));
                }
            }
        }

        Ok(())
    }
}
