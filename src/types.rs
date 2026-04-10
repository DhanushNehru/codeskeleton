//! Core types shared across all cograph modules.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;

/// Confidence level for an extracted relationship.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Confidence {
    /// Relationship found directly in source (e.g., import statement, direct call).
    Extracted,
    /// Reasonable inference (e.g., call-graph second pass, name matching).
    Inferred,
    /// Uncertain — flagged for review.
    Ambiguous,
}

impl fmt::Display for Confidence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Extracted => write!(f, "EXTRACTED"),
            Self::Inferred => write!(f, "INFERRED"),
            Self::Ambiguous => write!(f, "AMBIGUOUS"),
        }
    }
}

/// Kind of node in the knowledge graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    File,
    Class,
    Function,
    Method,
    Struct,
    Trait,
    Interface,
    Module,
    Enum,
}

impl fmt::Display for NodeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::File => write!(f, "file"),
            Self::Class => write!(f, "class"),
            Self::Function => write!(f, "function"),
            Self::Method => write!(f, "method"),
            Self::Struct => write!(f, "struct"),
            Self::Trait => write!(f, "trait"),
            Self::Interface => write!(f, "interface"),
            Self::Module => write!(f, "module"),
            Self::Enum => write!(f, "enum"),
        }
    }
}

/// Kind of edge (relationship) between nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeKind {
    Contains,
    Imports,
    ImportsFrom,
    Calls,
    Inherits,
    Implements,
    HasMethod,
}

impl fmt::Display for EdgeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Contains => write!(f, "contains"),
            Self::Imports => write!(f, "imports"),
            Self::ImportsFrom => write!(f, "imports_from"),
            Self::Calls => write!(f, "calls"),
            Self::Inherits => write!(f, "inherits"),
            Self::Implements => write!(f, "implements"),
            Self::HasMethod => write!(f, "method"),
        }
    }
}

/// A node extracted from source code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedNode {
    pub id: String,
    pub label: String,
    pub kind: NodeKind,
    pub source_file: String,
    pub line: usize,
}

/// An edge (relationship) extracted from source code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedEdge {
    pub source: String,
    pub target: String,
    pub kind: EdgeKind,
    pub confidence: Confidence,
    pub source_file: String,
    pub line: usize,
    pub weight: f64,
}

/// Complete extraction result for one or more files.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Extraction {
    pub nodes: Vec<ExtractedNode>,
    pub edges: Vec<ExtractedEdge>,
}

impl Extraction {
    pub fn new() -> Self {
        Self::default()
    }

    /// Merge another extraction into this one.
    pub fn merge(&mut self, other: Extraction) {
        self.nodes.extend(other.nodes);
        self.edges.extend(other.edges);
    }

    /// Deduplicate nodes by ID, keeping the first occurrence.
    pub fn dedup_nodes(&mut self) {
        let mut seen = HashSet::new();
        self.nodes.retain(|n| seen.insert(n.id.clone()));
    }
}

/// Build a stable, lowercase node ID from name parts.
///
/// Example: `make_id(&["utils", "parse_config"])` → `"utils_parse_config"`
pub fn make_id(parts: &[&str]) -> String {
    let combined: String = parts
        .iter()
        .filter(|p| !p.is_empty())
        .map(|p| p.trim_matches(|c: char| c == '_' || c == '.'))
        .collect::<Vec<_>>()
        .join("_");
    let cleaned: String = combined
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect();
    cleaned.trim_matches('_').to_lowercase()
}

/// Extract text from a tree-sitter node.
pub fn node_text<'a>(node: tree_sitter::Node, source: &'a [u8]) -> &'a str {
    let bytes = &source[node.start_byte()..node.end_byte()];
    std::str::from_utf8(bytes).unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make_id() {
        assert_eq!(make_id(&["utils", "parse_config"]), "utils_parse_config");
        assert_eq!(make_id(&["My.Module", "doStuff"]), "my_module_dostuff");
        assert_eq!(make_id(&["", "hello"]), "hello");
    }
}
