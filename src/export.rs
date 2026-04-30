//! Export — graph as JSON and interactive HTML visualization.

use crate::analyze::Analysis;
use crate::graph::KnowledgeGraph;
use petgraph::visit::EdgeRef;
use serde::Serialize;
use std::collections::HashMap;
use std::fmt::Write;
use std::fs;
use std::path::Path;

/// Serializable graph format for JSON export.
#[derive(Serialize)]
struct ExportGraph {
    nodes: Vec<ExportNode>,
    edges: Vec<ExportEdge>,
    communities: HashMap<usize, Vec<String>>,
    analysis: Analysis,
}

#[derive(Serialize)]
struct ExportNode {
    id: String,
    label: String,
    kind: String,
    source_file: String,
    line: usize,
    community: Option<usize>,
}

#[derive(Serialize)]
struct ExportEdge {
    source: String,
    target: String,
    kind: String,
    confidence: String,
    weight: f64,
}

/// Export the graph as JSON.
pub fn export_json(
    kg: &KnowledgeGraph,
    communities: &HashMap<usize, Vec<String>>,
    analysis: &Analysis,
    out_dir: &Path,
) -> std::io::Result<()> {
    fs::create_dir_all(out_dir)?;

    let export = build_export(kg, communities, analysis);
    let json = serde_json::to_string_pretty(&export)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    fs::write(out_dir.join("graph.json"), json)?;
    Ok(())
}

/// Export the graph as interactive HTML with vis.js.
pub fn export_html(
    kg: &KnowledgeGraph,
    communities: &HashMap<usize, Vec<String>>,
    analysis: &Analysis,
    out_dir: &Path,
) -> std::io::Result<()> {
    fs::create_dir_all(out_dir)?;

    let export = build_export(kg, communities, analysis);
    let graph_json = serde_json::to_string(&export)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    let html = generate_html(&export, &graph_json);
    fs::write(out_dir.join("graph.html"), html)?;
    Ok(())
}

fn build_export(
    kg: &KnowledgeGraph,
    communities: &HashMap<usize, Vec<String>>,
    analysis: &Analysis,
) -> ExportGraph {
    let nodes: Vec<ExportNode> = kg
        .graph
        .node_indices()
        .map(|idx| {
            let node = &kg.graph[idx];
            ExportNode {
                id: node.id.clone(),
                label: node.label.clone(),
                kind: node.kind.to_string(),
                source_file: node.source_file.clone(),
                line: node.line,
                community: node.community,
            }
        })
        .collect();

    let edges: Vec<ExportEdge> = kg
        .graph
        .edge_references()
        .map(|edge_ref| {
            let edge = edge_ref.weight();
            ExportEdge {
                source: kg.graph[edge_ref.source()].id.clone(),
                target: kg.graph[edge_ref.target()].id.clone(),
                kind: edge.kind.to_string(),
                confidence: edge.confidence.to_string(),
                weight: edge.weight,
            }
        })
        .collect();

    ExportGraph {
        nodes,
        edges,
        communities: communities.clone(),
        analysis: analysis.clone(),
    }
}

fn generate_html(export: &ExportGraph, _graph_json: &str) -> String {
    // Generate community color palette
    let community_count = export.communities.len().max(1);
    let mut colors = String::new();
    for i in 0..community_count {
        let hue = (i as f64 / community_count as f64 * 360.0) as u32;
        write!(colors, "'hsl({}, 70%, 55%)',", hue).unwrap();
    }

    // Build vis.js nodes
    let mut vis_nodes = String::from("[");
    for node in &export.nodes {
        let community = node.community.unwrap_or(0);
        let color_idx = community % community_count;
        let shape = match node.kind.as_str() {
            "file" => "diamond",
            "class" | "struct" | "trait" | "interface" | "enum" => "box",
            "function" | "method" => "ellipse",
            _ => "dot",
        };
        let size = match node.kind.as_str() {
            "file" => 25,
            "class" | "struct" => 20,
            _ => 15,
        };
        write!(
            vis_nodes,
            r#"{{id:"{}",label:"{}",shape:"{}",size:{},group:{},title:"{} ({}) — {}:L{}"}},"#,
            node.id,
            node.label.replace('"', r#"\""#),
            shape,
            size,
            color_idx,
            node.label.replace('"', r#"\""#),
            node.kind,
            node.source_file.replace('"', r#"\""#),
            node.line
        )
        .unwrap();
    }
    vis_nodes.push(']');

    // Build vis.js edges
    let mut vis_edges = String::from("[");
    for edge in &export.edges {
        let dashes = if edge.confidence == "INFERRED" {
            "true"
        } else {
            "false"
        };
        let edge_color = match edge.kind.as_str() {
            "imports" | "imports_from" => "'#666'",
            "calls" => "'#e74c3c'",
            "inherits" | "implements" => "'#9b59b6'",
            "contains" | "method" => "'#95a5a6'",
            _ => "'#bdc3c7'",
        };
        write!(
            vis_edges,
            r#"{{from:"{}",to:"{}",color:{{color:{}}},dashes:{},title:"{} ({})",arrows:"to"}},"#,
            edge.source, edge.target, edge_color, dashes, edge.kind, edge.confidence,
        )
        .unwrap();
    }
    vis_edges.push(']');

    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>codeskeleton — Knowledge Graph</title>
    <script src="https://unpkg.com/vis-network@9.1.6/standalone/umd/vis-network.min.js"></script>
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: #0a0a0a;
            color: #e0e0e0;
            overflow: hidden;
        }}
        #header {{
            position: fixed;
            top: 0;
            left: 0;
            right: 0;
            z-index: 100;
            background: rgba(10, 10, 10, 0.95);
            backdrop-filter: blur(10px);
            padding: 12px 24px;
            display: flex;
            align-items: center;
            gap: 16px;
            border-bottom: 1px solid #222;
        }}
        #header h1 {{
            font-size: 18px;
            font-weight: 600;
            background: linear-gradient(135deg, #00d2ff, #3a7bd5);
            -webkit-background-clip: text;
            background-clip: text;
            -webkit-text-fill-color: transparent;
        }}
        #search {{
            padding: 8px 16px;
            background: #1a1a1a;
            border: 1px solid #333;
            border-radius: 8px;
            color: #e0e0e0;
            font-size: 14px;
            width: 300px;
            outline: none;
            transition: border-color 0.2s;
        }}
        #search:focus {{ border-color: #3a7bd5; }}
        .stats {{
            font-size: 13px;
            color: #888;
            margin-left: auto;
        }}
        .stats span {{ color: #00d2ff; font-weight: 600; }}
        #graph {{
            width: 100vw;
            height: 100vh;
            padding-top: 56px;
        }}
        #info-panel {{
            position: fixed;
            bottom: 20px;
            right: 20px;
            background: rgba(26, 26, 26, 0.95);
            backdrop-filter: blur(10px);
            border: 1px solid #333;
            border-radius: 12px;
            padding: 16px;
            max-width: 350px;
            display: none;
            z-index: 100;
            font-size: 13px;
            line-height: 1.6;
        }}
        #info-panel h3 {{
            font-size: 15px;
            margin-bottom: 8px;
            color: #00d2ff;
        }}
        #info-panel .label {{ color: #888; }}
        #info-panel .value {{ color: #e0e0e0; }}
    </style>
</head>
<body>
    <div id="header">
        <h1>⬡ codeskeleton</h1>
        <input type="text" id="search" placeholder="Search nodes..." />
        <div class="stats">
            <span>{node_count}</span> nodes · <span>{edge_count}</span> edges · <span>{community_count}</span> communities
        </div>
    </div>
    <div id="graph"></div>
    <div id="info-panel">
        <h3 id="info-title"></h3>
        <div id="info-content"></div>
    </div>

    <script>
        const nodes = new vis.DataSet({vis_nodes});
        const edges = new vis.DataSet({vis_edges});
        const colors = [{colors}];

        const container = document.getElementById('graph');
        const network = new vis.Network(container, {{ nodes, edges }}, {{
            physics: {{
                solver: 'forceAtlas2Based',
                forceAtlas2Based: {{ gravitationalConstant: -50, springLength: 100, springConstant: 0.08 }},
                stabilization: {{ iterations: 200 }}
            }},
            interaction: {{ hover: true, tooltipDelay: 100, zoomView: true }},
            groups: {{}},
            edges: {{ smooth: {{ type: 'continuous' }}, width: 1 }}
        }});

        // Search
        document.getElementById('search').addEventListener('input', function(e) {{
            const query = e.target.value.toLowerCase();
            if (!query) {{
                nodes.forEach(n => nodes.update({{ id: n.id, hidden: false }}));
                return;
            }}
            nodes.forEach(n => {{
                const match = n.label.toLowerCase().includes(query) || n.id.toLowerCase().includes(query);
                nodes.update({{ id: n.id, hidden: !match, opacity: match ? 1 : 0.1 }});
            }});
        }});

        // Click info panel
        network.on('click', function(params) {{
            const panel = document.getElementById('info-panel');
            if (params.nodes.length > 0) {{
                const nodeId = params.nodes[0];
                const node = nodes.get(nodeId);
                document.getElementById('info-title').textContent = node.label;
                document.getElementById('info-content').innerHTML =
                    '<span class="label">Kind:</span> <span class="value">' + (node.title || '') + '</span>';
                panel.style.display = 'block';
            }} else {{
                panel.style.display = 'none';
            }}
        }});
    </script>
</body>
</html>"##,
        vis_nodes = vis_nodes,
        vis_edges = vis_edges,
        colors = colors,
        node_count = export.nodes.len(),
        edge_count = export.edges.len(),
        community_count = export.communities.len(),
    )
}
