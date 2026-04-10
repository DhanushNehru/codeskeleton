//! AST extraction engine — generic tree-sitter walker driven by LanguageSpec.

use crate::languages::{LanguageSpec, SupportedLanguage};
use crate::types::*;
use std::collections::HashSet;
use std::path::Path;
use tree_sitter::Parser;

/// Extract nodes and edges from a single source file.
pub fn extract_file(path: &Path, lang: SupportedLanguage) -> Result<Extraction, String> {
    let spec = lang.spec();

    let mut parser = Parser::new();
    parser
        .set_language(&lang.ts_language())
        .map_err(|e| format!("Failed to set language for {}: {}", lang.name(), e))?;

    let source = std::fs::read(path).map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    let tree = parser
        .parse(&source, None)
        .ok_or_else(|| format!("Failed to parse {}", path.display()))?;
    let root = tree.root_node();

    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");
    let file_path = path.to_string_lossy().to_string();

    let mut extraction = Extraction::new();
    let mut seen = HashSet::new();

    // File-level node
    let file_nid = make_id(&[stem]);
    seen.insert(file_nid.clone());
    extraction.nodes.push(ExtractedNode {
        id: file_nid.clone(),
        label: path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string(),
        kind: NodeKind::File,
        source_file: file_path.clone(),
        line: 1,
    });

    // Collect function bodies for call-graph second pass.
    // We store byte ranges instead of Node references to avoid lifetime issues.
    let mut func_bodies: Vec<(String, usize, usize)> = Vec::new();

    // First pass: structural extraction
    walk_node(
        root,
        &source,
        &spec,
        &file_nid,
        stem,
        &file_path,
        None,
        lang,
        &mut extraction,
        &mut seen,
        &mut func_bodies,
    );

    // Second pass: call graph
    for (func_nid, start_byte, end_byte) in &func_bodies {
        walk_calls_in_range(
            root,
            &source,
            &spec,
            func_nid,
            *start_byte,
            *end_byte,
            &seen,
            &file_path,
            &mut extraction,
        );
    }

    Ok(extraction)
}

/// Recursively walk the AST extracting classes, functions, and imports.
fn walk_node(
    node: tree_sitter::Node,
    source: &[u8],
    spec: &LanguageSpec,
    file_nid: &str,
    stem: &str,
    file_path: &str,
    parent_class: Option<&str>,
    lang: SupportedLanguage,
    extraction: &mut Extraction,
    seen: &mut HashSet<String>,
    func_bodies: &mut Vec<(String, usize, usize)>,
) {
    let kind = node.kind();

    // ── Imports ──────────────────────────────────────────────────────
    if spec.import_types.contains(&kind) {
        if let Some(target) = extract_import(node, source, lang) {
            let target_nid = make_id(&[&target]);
            let line = node.start_position().row + 1;
            extraction.edges.push(ExtractedEdge {
                source: file_nid.to_string(),
                target: target_nid,
                kind: EdgeKind::Imports,
                confidence: Confidence::Extracted,
                source_file: file_path.to_string(),
                line,
                weight: 1.0,
            });
        }
        return;
    }

    // ── Classes / Structs / Traits ───────────────────────────────────
    if spec.class_types.contains(&kind) {
        if let Some(name) = resolve_name(node, source, spec) {
            let class_nid = make_id(&[stem, &name]);
            let line = node.start_position().row + 1;

            let node_kind = match kind {
                "struct_item" | "struct_specifier" => NodeKind::Struct,
                "trait_item" => NodeKind::Trait,
                "interface_declaration" => NodeKind::Interface,
                "enum_item" => NodeKind::Enum,
                "type_declaration" => NodeKind::Struct, // Go types
                _ => NodeKind::Class,
            };

            if seen.insert(class_nid.clone()) {
                extraction.nodes.push(ExtractedNode {
                    id: class_nid.clone(),
                    label: name.clone(),
                    kind: node_kind,
                    source_file: file_path.to_string(),
                    line,
                });
                extraction.edges.push(ExtractedEdge {
                    source: file_nid.to_string(),
                    target: class_nid.clone(),
                    kind: EdgeKind::Contains,
                    confidence: Confidence::Extracted,
                    source_file: file_path.to_string(),
                    line,
                    weight: 1.0,
                });
            }

            // Recurse into body
            if let Some(body) = find_body(node, spec) {
                let mut cursor = body.walk();
                for child in body.children(&mut cursor) {
                    walk_node(
                        child,
                        source,
                        spec,
                        file_nid,
                        stem,
                        file_path,
                        Some(&class_nid),
                        lang,
                        extraction,
                        seen,
                        func_bodies,
                    );
                }
            }
        }
        return;
    }

    // ── Functions / Methods ─────────────────────────────────────────
    if spec.function_types.contains(&kind) {
        if let Some(name) = resolve_name(node, source, spec) {
            let line = node.start_position().row + 1;
            let (func_nid, label, edge_kind) = if let Some(parent) = parent_class {
                let nid = make_id(&[parent, &name]);
                let label = format!(".{}()", name);
                (nid, label, EdgeKind::HasMethod)
            } else {
                let nid = make_id(&[stem, &name]);
                let label = if spec.function_label_parens {
                    format!("{}()", name)
                } else {
                    name.clone()
                };
                (nid, label, EdgeKind::Contains)
            };

            let node_kind = if parent_class.is_some() {
                NodeKind::Method
            } else {
                NodeKind::Function
            };

            if seen.insert(func_nid.clone()) {
                extraction.nodes.push(ExtractedNode {
                    id: func_nid.clone(),
                    label,
                    kind: node_kind,
                    source_file: file_path.to_string(),
                    line,
                });

                let parent_nid = parent_class.unwrap_or(file_nid).to_string();
                extraction.edges.push(ExtractedEdge {
                    source: parent_nid,
                    target: func_nid.clone(),
                    kind: edge_kind,
                    confidence: Confidence::Extracted,
                    source_file: file_path.to_string(),
                    line,
                    weight: 1.0,
                });
            }

            // Save body range for call-graph pass
            if let Some(body) = find_body(node, spec) {
                func_bodies.push((func_nid, body.start_byte(), body.end_byte()));
            }
        }
        return;
    }

    // ── Recurse into children ───────────────────────────────────────
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_node(
            child, source, spec, file_nid, stem, file_path, parent_class, lang, extraction,
            seen, func_bodies,
        );
    }
}

/// Resolve the name of a definition node.
fn resolve_name(node: tree_sitter::Node, source: &[u8], spec: &LanguageSpec) -> Option<String> {
    // Try the primary name field
    if let Some(name_node) = node.child_by_field_name(spec.name_field) {
        let text = node_text(name_node, source);
        if !text.is_empty() {
            return Some(text.to_string());
        }
    }
    // Fallback: look for specific child types
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if spec.name_fallback_types.contains(&child.kind()) {
            let text = node_text(child, source);
            if !text.is_empty() {
                return Some(text.to_string());
            }
        }
    }
    None
}

/// Find the body node of a definition.
fn find_body<'a>(node: tree_sitter::Node<'a>, spec: &LanguageSpec) -> Option<tree_sitter::Node<'a>> {
    if let Some(body) = node.child_by_field_name(spec.body_field) {
        return Some(body);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if spec.body_fallback_types.contains(&child.kind()) {
            return Some(child);
        }
    }
    None
}

/// Extract the import target module name from an import node.
fn extract_import(node: tree_sitter::Node, source: &[u8], lang: SupportedLanguage) -> Option<String> {
    match lang {
        SupportedLanguage::Python => extract_python_import(node, source),
        SupportedLanguage::JavaScript | SupportedLanguage::TypeScript | SupportedLanguage::Tsx => {
            extract_js_import(node, source)
        }
        SupportedLanguage::Rust => extract_rust_import(node, source),
        SupportedLanguage::Go => extract_go_import(node, source),
        SupportedLanguage::Java => extract_java_import(node, source),
        SupportedLanguage::C => extract_c_import(node, source),
    }
}

fn extract_python_import(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    let text = node_text(node, source);
    if node.kind() == "import_from_statement" {
        // `from X import Y` — extract X
        if let Some(module_node) = node.child_by_field_name("module_name") {
            let module = node_text(module_node, source).trim_start_matches('.').to_string();
            if !module.is_empty() {
                return Some(module.split('.').last().unwrap_or(&module).to_string());
            }
        }
    }
    // `import X` or `import X.Y` — extract last component
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "dotted_name" || child.kind() == "aliased_import" {
            let raw = node_text(child, source);
            let clean = raw.split(" as ").next().unwrap_or(raw).trim();
            let module = clean.split('.').last().unwrap_or(clean).trim_start_matches('.');
            if !module.is_empty() {
                return Some(module.to_string());
            }
        }
    }
    // Fallback: extract from text
    let parts: Vec<&str> = text.split_whitespace().collect();
    parts.get(1).map(|s| s.split('.').last().unwrap_or(s).to_string())
}

fn extract_js_import(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    // Look for the string source in import statements
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "string" || child.kind() == "string_fragment" {
            let raw = node_text(child, source)
                .trim_matches(|c: char| c == '\'' || c == '"' || c == '`' || c == ' ');
            let module = raw
                .trim_start_matches("./")
                .trim_start_matches("../")
                .split('/')
                .last()
                .unwrap_or(raw);
            if !module.is_empty() {
                return Some(module.to_string());
            }
        }
    }
    None
}

fn extract_rust_import(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    // `use X::Y::Z;` — extract the last meaningful component
    let text = node_text(node, source);
    let cleaned = text
        .trim_start_matches("use ")
        .trim_end_matches(';')
        .trim();
    // Handle `use crate::foo::bar;` → "bar"
    // Handle `use std::collections::HashMap;` → "HashMap"
    let last = cleaned
        .split("::")
        .last()
        .unwrap_or(cleaned)
        .trim_matches(|c: char| c == '{' || c == '}' || c == ' ' || c == '*');
    if !last.is_empty() && last != "self" {
        Some(last.to_string())
    } else {
        // If it's a glob or self, use the parent module
        let parts: Vec<&str> = cleaned.split("::").collect();
        parts.iter().rev().nth(1).map(|s| s.trim().to_string())
    }
}

fn extract_go_import(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    // Go import: `import "fmt"` or `import ("fmt"; "net/http")`
    fn find_strings(node: tree_sitter::Node, source: &[u8]) -> Vec<String> {
        let mut results = Vec::new();
        if node.kind() == "interpreted_string_literal" {
            let raw = node_text(node, source).trim_matches('"');
            if let Some(pkg) = raw.split('/').last() {
                if !pkg.is_empty() {
                    results.push(pkg.to_string());
                }
            }
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            results.extend(find_strings(child, source));
        }
        results
    }
    let strings = find_strings(node, source);
    strings.into_iter().next()
}

fn extract_java_import(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    // `import com.example.Foo;` → "Foo"
    let text = node_text(node, source);
    let cleaned = text
        .trim_start_matches("import ")
        .trim_start_matches("static ")
        .trim_end_matches(';')
        .trim();
    let last = cleaned.split('.').last().unwrap_or(cleaned).trim();
    if !last.is_empty() && last != "*" {
        Some(last.to_string())
    } else {
        // For wildcard imports, use the parent package
        let parts: Vec<&str> = cleaned.split('.').collect();
        parts.iter().rev().nth(1).map(|s| s.trim().to_string())
    }
}

fn extract_c_import(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    // `#include <stdio.h>` or `#include "mylib.h"`
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "string_literal"
            || child.kind() == "system_lib_string"
            || child.kind() == "string"
        {
            let raw = node_text(child, source)
                .trim_matches(|c: char| c == '"' || c == '<' || c == '>' || c == ' ');
            let module = raw.split('/').last().unwrap_or(raw);
            let name = module.split('.').next().unwrap_or(module);
            if !name.is_empty() {
                return Some(name.to_string());
            }
        }
    }
    None
}

/// Walk a byte range looking for call expressions (second pass for call graph).
fn walk_calls_in_range(
    node: tree_sitter::Node,
    source: &[u8],
    spec: &LanguageSpec,
    func_nid: &str,
    start_byte: usize,
    end_byte: usize,
    known_functions: &HashSet<String>,
    file_path: &str,
    extraction: &mut Extraction,
) {
    // Only process nodes within the byte range
    if node.end_byte() <= start_byte || node.start_byte() >= end_byte {
        return;
    }

    if spec.call_types.contains(&node.kind()) {
        if let Some(callee_node) = node.child_by_field_name(spec.call_function_field) {
            let callee_text = node_text(callee_node, source);
            // Extract the final name (e.g., `obj.method` → `method`, `foo` → `foo`)
            let callee_name = callee_text
                .rsplit(|c: char| c == '.' || c == ':')
                .next()
                .unwrap_or(callee_text)
                .trim();

            if !callee_name.is_empty() {
                let callee_id = make_id(&[callee_name]);
                // Only create edge if target function is known
                let line = node.start_position().row + 1;

                // Check if any known function ID ends with the callee name
                let target_match = known_functions
                    .iter()
                    .find(|id| id.ends_with(&callee_id) || **id == callee_id);

                if let Some(target) = target_match {
                    extraction.edges.push(ExtractedEdge {
                        source: func_nid.to_string(),
                        target: target.clone(),
                        kind: EdgeKind::Calls,
                        confidence: Confidence::Inferred,
                        source_file: file_path.to_string(),
                        line,
                        weight: 0.8,
                    });
                }
            }
        }
    }

    // Recurse into children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_calls_in_range(
            child, source, spec, func_nid, start_byte, end_byte, known_functions, file_path,
            extraction,
        );
    }
}
