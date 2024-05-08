use std::path::Path;

use crate::{Error, Result};
use serde::{Deserialize, Serialize};

/// A single node in the assembly tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    link: Option<String>,
    label: String,

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
    pub fn get_children(&self) -> &Vec<usize> {
        &self.children
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
        let rdr = std::io::BufReader::new(std::fs::File::open(file).unwrap());
        let assembly: Assembly = serde_json::from_reader(rdr)
            .map_err(|e| Error::IO(format!("Failed to load assembly from file: {}", e)))?;

        assembly.is_valid()?;

        Ok(assembly)
    }

    /// Checks if the assembly is valid.
    pub fn is_valid(&self) -> Result<()> {
        let num_nodes = self.nodes.len();

        for node in self.nodes.iter() {
            for child in node.get_children() {
                if *child >= num_nodes {
                    return Err(Error::InvalidFormat(format!(
                        "Invalid child index {} in node {}",
                        child,
                        node.get_label()
                    )));
                }
            }
        }

        Ok(())
    }
}
