//! codeskeleton — turn any folder of code into a queryable knowledge graph.
//!
//! Single binary, zero runtime dependencies, blazing fast AST extraction
//! via tree-sitter with parallel processing.

pub mod analyze;
pub mod cache;
pub mod cluster;
pub mod detect;
pub mod export;
pub mod extract;
pub mod graph;
pub mod languages;
pub mod report;
pub mod types;
