//! Build a petgraph graph from extraction results.

use crate::types::*;
use petgraph::graph::{Graph, NodeIndex};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Node data stored in the petgraph graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub kind: NodeKind,
    pub source_file: String,
    pub line: usize,
    pub community: Option<usize>,
}

/// Edge data stored in the petgraph graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub kind: EdgeKind,
    pub confidence: Confidence,
    pub source_file: String,
    pub line: usize,
    pub weight: f64,
}

/// The assembled knowledge graph.
pub struct KnowledgeGraph {
    pub graph: Graph<GraphNode, GraphEdge, petgraph::Undirected>,
    pub node_map: HashMap<String, NodeIndex>,
}

impl KnowledgeGraph {
    /// Build a knowledge graph from extraction results.
    pub fn from_extractions(extractions: &[Extraction]) -> Self {
        let mut graph = Graph::new_undirected();
        let mut node_map: HashMap<String, NodeIndex> = HashMap::new();

        // Add all nodes
        for ext in extractions {
            for node in &ext.nodes {
                if !node_map.contains_key(&node.id) {
                    let idx = graph.add_node(GraphNode {
                        id: node.id.clone(),
                        label: node.label.clone(),
                        kind: node.kind,
                        source_file: node.source_file.clone(),
                        line: node.line,
                        community: None,
                    });
                    node_map.insert(node.id.clone(), idx);
                }
            }
        }

        // Add edges (skip dangling — edges to external/stdlib nodes)
        for ext in extractions {
            for edge in &ext.edges {
                if let (Some(&src_idx), Some(&tgt_idx)) =
                    (node_map.get(&edge.source), node_map.get(&edge.target))
                {
                    // Avoid self-loops
                    if src_idx != tgt_idx {
                        graph.add_edge(
                            src_idx,
                            tgt_idx,
                            GraphEdge {
                                kind: edge.kind,
                                confidence: edge.confidence,
                                source_file: edge.source_file.clone(),
                                line: edge.line,
                                weight: edge.weight,
                            },
                        );
                    }
                }
            }
        }

        Self { graph, node_map }
    }

    /// Get a node by its ID.
    pub fn get_node(&self, id: &str) -> Option<&GraphNode> {
        self.node_map
            .get(id)
            .map(|&idx| &self.graph[idx])
    }

    /// Get node count.
    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    /// Get edge count.
    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    /// Set community labels on nodes.
    pub fn set_communities(&mut self, communities: &HashMap<usize, Vec<String>>) {
        for (community_id, node_ids) in communities {
            for node_id in node_ids {
                if let Some(&idx) = self.node_map.get(node_id) {
                    self.graph[idx].community = Some(*community_id);
                }
            }
        }
    }
}
