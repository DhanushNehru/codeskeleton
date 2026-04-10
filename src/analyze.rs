//! Graph analysis — god nodes, surprising connections, suggested questions.

use crate::cluster::cohesion_score;
use crate::graph::KnowledgeGraph;
use petgraph::visit::EdgeRef;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Complete analysis results for the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Analysis {
    pub god_nodes: Vec<GodNode>,
    pub surprising_connections: Vec<SurprisingConnection>,
    pub suggested_questions: Vec<String>,
    pub stats: GraphStats,
    pub community_scores: HashMap<usize, f64>,
}

/// A high-degree node that many things connect through.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GodNode {
    pub id: String,
    pub label: String,
    pub degree: usize,
    pub kind: String,
    pub community: Option<usize>,
}

/// A cross-community connection that may reveal hidden structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurprisingConnection {
    pub source: String,
    pub target: String,
    pub source_community: Option<usize>,
    pub target_community: Option<usize>,
    pub relation: String,
    pub why: String,
}

/// High-level graph statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStats {
    pub total_nodes: usize,
    pub total_edges: usize,
    pub total_communities: usize,
    pub files_analyzed: usize,
}

/// Run full analysis on the knowledge graph.
pub fn analyze(
    kg: &KnowledgeGraph,
    communities: &HashMap<usize, Vec<String>>,
) -> Analysis {
    let god_nodes = find_god_nodes(kg, 10);
    let surprising_connections = find_surprising_connections(kg, communities, 10);
    let suggested_questions = generate_questions(&god_nodes, communities);
    let community_scores = compute_community_scores(kg, communities);

    let files_analyzed = kg
        .graph
        .node_indices()
        .filter(|&idx| {
            matches!(kg.graph[idx].kind, crate::types::NodeKind::File)
        })
        .count();

    let stats = GraphStats {
        total_nodes: kg.node_count(),
        total_edges: kg.edge_count(),
        total_communities: communities.len(),
        files_analyzed,
    };

    Analysis {
        god_nodes,
        surprising_connections,
        suggested_questions,
        stats,
        community_scores,
    }
}

/// Find the top-N highest-degree nodes.
fn find_god_nodes(kg: &KnowledgeGraph, limit: usize) -> Vec<GodNode> {
    let mut nodes: Vec<GodNode> = kg
        .graph
        .node_indices()
        .map(|idx| {
            let node = &kg.graph[idx];
            let degree = kg.graph.edges(idx).count();
            GodNode {
                id: node.id.clone(),
                label: node.label.clone(),
                degree,
                kind: node.kind.to_string(),
                community: node.community,
            }
        })
        .collect();

    nodes.sort_by(|a, b| b.degree.cmp(&a.degree));
    nodes.truncate(limit);
    nodes
}

/// Find cross-community edges — connections between different communities.
fn find_surprising_connections(
    kg: &KnowledgeGraph,
    communities: &HashMap<usize, Vec<String>>,
    limit: usize,
) -> Vec<SurprisingConnection> {
    // Build reverse map: node_id → community_id
    let mut node_community: HashMap<&str, usize> = HashMap::new();
    for (cid, nodes) in communities {
        for node_id in nodes {
            node_community.insert(node_id.as_str(), *cid);
        }
    }

    let mut connections: Vec<SurprisingConnection> = Vec::new();

    for edge_ref in kg.graph.edge_references() {
        let source = &kg.graph[edge_ref.source()];
        let target = &kg.graph[edge_ref.target()];
        let edge = edge_ref.weight();

        let src_community = node_community.get(source.id.as_str()).copied();
        let tgt_community = node_community.get(target.id.as_str()).copied();

        // Only interested in cross-community edges
        if src_community != tgt_community {
            let why = format!(
                "{} ({}) connects to {} ({}) via {} — bridging community {} and {}",
                source.label,
                source.kind,
                target.label,
                target.kind,
                edge.kind,
                src_community.map_or("?".to_string(), |c| c.to_string()),
                tgt_community.map_or("?".to_string(), |c| c.to_string()),
            );

            connections.push(SurprisingConnection {
                source: source.id.clone(),
                target: target.id.clone(),
                source_community: src_community,
                target_community: tgt_community,
                relation: edge.kind.to_string(),
                why,
            });
        }
    }

    // Sort by cross-community distance (higher = more surprising)
    connections.sort_by(|a, b| {
        let dist_a = community_distance(a.source_community, a.target_community);
        let dist_b = community_distance(b.source_community, b.target_community);
        dist_b.partial_cmp(&dist_a).unwrap_or(std::cmp::Ordering::Equal)
    });
    connections.truncate(limit);
    connections
}

fn community_distance(a: Option<usize>, b: Option<usize>) -> f64 {
    match (a, b) {
        (Some(a), Some(b)) => (a as f64 - b as f64).abs() + 1.0,
        _ => 0.5,
    }
}

/// Generate suggested questions from graph structure.
fn generate_questions(
    god_nodes: &[GodNode],
    communities: &HashMap<usize, Vec<String>>,
) -> Vec<String> {
    let mut questions = Vec::new();

    if let Some(top) = god_nodes.first() {
        questions.push(format!(
            "Why does '{}' have {} connections? What role does it play in the architecture?",
            top.label, top.degree
        ));
    }

    if god_nodes.len() >= 2 {
        questions.push(format!(
            "What is the relationship between '{}' and '{}'?",
            god_nodes[0].label, god_nodes[1].label
        ));
    }

    if communities.len() > 1 {
        questions.push(format!(
            "What bridges the {} communities together? Are there clear module boundaries?",
            communities.len()
        ));
    }

    if god_nodes.len() >= 3 {
        questions.push(format!(
            "Could '{}' be refactored to reduce its {} connections?",
            god_nodes[0].label, god_nodes[0].degree
        ));
    }

    questions.push("What design patterns are used across the codebase?".to_string());
    questions.truncate(5);
    questions
}

/// Compute cohesion scores for all communities.
fn compute_community_scores(
    kg: &KnowledgeGraph,
    communities: &HashMap<usize, Vec<String>>,
) -> HashMap<usize, f64> {
    communities
        .iter()
        .map(|(cid, nodes)| (*cid, cohesion_score(kg, nodes)))
        .collect()
}
