//! Community detection via label propagation.
//!
//! Simple, fast, zero-dependency algorithm. Each node starts in its own
//! community, then iteratively adopts the most common community among
//! its neighbors. Oversized communities (>25% of graph) get split.

use crate::graph::KnowledgeGraph;
use petgraph::visit::EdgeRef;
use std::collections::HashMap;

const MAX_ITERATIONS: usize = 50;
const MAX_COMMUNITY_FRACTION: f64 = 0.25;
const MIN_SPLIT_SIZE: usize = 10;

/// Run label propagation community detection.
///
/// Returns `{ community_id: [node_ids] }` sorted by size descending.
pub fn cluster(kg: &KnowledgeGraph) -> HashMap<usize, Vec<String>> {
    let graph = &kg.graph;
    let n = graph.node_count();
    if n == 0 {
        return HashMap::new();
    }

    // Initialize: each node in its own community
    let mut labels: HashMap<petgraph::graph::NodeIndex, usize> = HashMap::new();
    let indices: Vec<_> = graph.node_indices().collect();
    for (i, &idx) in indices.iter().enumerate() {
        labels.insert(idx, i);
    }

    // Iterate until convergence or max iterations
    for _ in 0..MAX_ITERATIONS {
        let mut changed = false;

        for &idx in &indices {
            // Count neighbor community labels
            let mut neighbor_counts: HashMap<usize, usize> = HashMap::new();
            for edge in graph.edges(idx) {
                let neighbor = edge.target();
                if neighbor == idx {
                    // Also check source for undirected
                    continue;
                }
                if let Some(&label) = labels.get(&neighbor) {
                    *neighbor_counts.entry(label).or_insert(0) += 1;
                }
            }

            // Adopt the most common neighbor label
            if let Some((&best_label, _)) = neighbor_counts.iter().max_by_key(|(_, &count)| count)
            {
                if labels.get(&idx) != Some(&best_label) {
                    labels.insert(idx, best_label);
                    changed = true;
                }
            }
        }

        if !changed {
            break;
        }
    }

    // Group nodes by community label
    let mut raw_communities: HashMap<usize, Vec<String>> = HashMap::new();
    for (&idx, &label) in &labels {
        let node_id = &graph[idx].id;
        raw_communities
            .entry(label)
            .or_default()
            .push(node_id.clone());
    }

    // Split oversized communities
    let max_size = std::cmp::max(MIN_SPLIT_SIZE, (n as f64 * MAX_COMMUNITY_FRACTION) as usize);
    let mut final_communities: Vec<Vec<String>> = Vec::new();
    for nodes in raw_communities.into_values() {
        if nodes.len() > max_size {
            // Simple split: divide into chunks
            let chunk_size = max_size;
            for chunk in nodes.chunks(chunk_size) {
                final_communities.push(chunk.to_vec());
            }
        } else {
            final_communities.push(nodes);
        }
    }

    // Sort by size descending and re-index
    final_communities.sort_by(|a, b| b.len().cmp(&a.len()));
    final_communities
        .into_iter()
        .enumerate()
        .map(|(i, mut nodes)| {
            nodes.sort();
            (i, nodes)
        })
        .collect()
}

/// Compute cohesion score for a community.
///
/// Ratio of actual intra-community edges to maximum possible.
pub fn cohesion_score(kg: &KnowledgeGraph, community_nodes: &[String]) -> f64 {
    let n = community_nodes.len();
    if n <= 1 {
        return 1.0;
    }

    let node_set: std::collections::HashSet<&str> =
        community_nodes.iter().map(|s| s.as_str()).collect();
    let mut actual_edges = 0usize;

    for node_id in community_nodes {
        if let Some(&idx) = kg.node_map.get(node_id) {
            for edge in kg.graph.edges(idx) {
                let neighbor_id = &kg.graph[edge.target()].id;
                if node_set.contains(neighbor_id.as_str()) {
                    actual_edges += 1;
                }
            }
        }
    }

    // Each undirected edge counted twice
    actual_edges /= 2;
    let possible = n * (n - 1) / 2;
    if possible == 0 {
        return 0.0;
    }
    (actual_edges as f64 / possible as f64 * 100.0).round() / 100.0
}
