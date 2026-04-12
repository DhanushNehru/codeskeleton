//! codeskeleton CLI — turn any folder of code into a queryable knowledge graph.

use clap::Parser as ClapParser;
use colored::Colorize;
use rayon::prelude::*;
use std::path::PathBuf;
use std::time::Instant;

use codeskeleton::analyze;
use codeskeleton::cache;
use codeskeleton::cluster;
use codeskeleton::detect;
use codeskeleton::export;
use codeskeleton::extract;
use codeskeleton::graph::KnowledgeGraph;
use codeskeleton::languages::SupportedLanguage;
use codeskeleton::report;
use codeskeleton::types::Extraction;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const OUTPUT_DIR: &str = "codeskeleton-out";
const CACHE_DIR: &str = "codeskeleton-out/cache";

#[derive(ClapParser)]
#[command(
    name = "codeskeleton",
    about = "Turn any folder of code into a queryable knowledge graph",
    version = VERSION,
    after_help = "Examples:\n  codeskeleton .              Analyze the current directory\n  codeskeleton ./src          Analyze a specific folder\n  codeskeleton . --no-cache   Force full re-extraction"
)]
struct Cli {
    /// Path to the directory to analyze.
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Force full re-extraction (ignore cache).
    #[arg(long)]
    no_cache: bool,

    /// Output formats (comma-separated: json, html, md).
    #[arg(long, value_delimiter = ',', default_value = "json,html,md")]
    format: Vec<String>,
}

fn main() {
    let cli = Cli::parse();
    let start = Instant::now();

    println!(
        "\n  {} {} {}\n",
        "⬡".bright_cyan(),
        "codeskeleton".bold(),
        format!("v{}", VERSION).dimmed()
    );

    let root = cli.path.canonicalize().unwrap_or_else(|_| {
        eprintln!(
            "  {} Path not found: {}",
            "✗".red(),
            cli.path.display()
        );
        std::process::exit(1);
    });

    // ── Step 1: Detect files ────────────────────────────────────────
    let all_files = detect::collect_files(&root);
    if all_files.is_empty() {
        eprintln!("  {} No code files found in {}", "✗".red(), root.display());
        std::process::exit(1);
    }
    println!(
        "  {} Found {} code files",
        "→".bright_cyan(),
        all_files.len().to_string().bold()
    );

    // ── Step 2: Cache check ─────────────────────────────────────────
    let cache_dir = root.join(CACHE_DIR);
    let (files_to_extract, manifest) = if cli.no_cache {
        (all_files.clone(), cache::CacheManifest::new())
    } else {
        cache::check_cache(&all_files, &cache_dir)
    };

    let cached_count = all_files.len() - files_to_extract.len();
    if cached_count > 0 {
        println!(
            "  {} {} files cached, extracting {}",
            "→".bright_cyan(),
            cached_count.to_string().dimmed(),
            files_to_extract.len().to_string().bold()
        );
    }

    // ── Step 3: Parallel extraction ─────────────────────────────────
    let extract_start = Instant::now();
    let extractions: Vec<Extraction> = files_to_extract
        .par_iter()
        .filter_map(|path| {
            let ext = path.extension()?.to_str()?;
            let lang = SupportedLanguage::from_extension(ext)?;
            match extract::extract_file(path, lang) {
                Ok(extraction) => Some(extraction),
                Err(e) => {
                    eprintln!("  {} {}: {}", "⚠".yellow(), path.display(), e);
                    None
                }
            }
        })
        .collect();

    let total_nodes: usize = extractions.iter().map(|e| e.nodes.len()).sum();
    let total_edges: usize = extractions.iter().map(|e| e.edges.len()).sum();
    let extract_time = extract_start.elapsed();

    println!(
        "  {} Extracted {} nodes, {} edges in {:.1}s",
        "✓".green(),
        total_nodes.to_string().bold(),
        total_edges.to_string().bold(),
        extract_time.as_secs_f64()
    );

    // ── Step 4: Build graph ─────────────────────────────────────────
    let mut kg = KnowledgeGraph::from_extractions(&extractions);
    println!(
        "  {} Graph: {} nodes, {} edges",
        "✓".green(),
        kg.node_count().to_string().bold(),
        kg.edge_count().to_string().bold()
    );

    // ── Step 5: Community detection ─────────────────────────────────
    let communities = cluster::cluster(&kg);
    kg.set_communities(&communities);
    println!(
        "  {} {} communities detected",
        "✓".green(),
        communities.len().to_string().bold()
    );

    // ── Step 6: Analysis ────────────────────────────────────────────
    let analysis = analyze::analyze(&kg, &communities);

    if let Some(top) = analysis.god_nodes.first() {
        println!(
            "  {} God node: {} ({} connections)",
            "★".bright_yellow(),
            top.label.bold(),
            top.degree
        );
    }

    // ── Step 7: Export ──────────────────────────────────────────────
    let out_dir = root.join(OUTPUT_DIR);
    let formats = &cli.format;

    // JSON
    if formats.contains(&"json".to_string()) {
        export::export_json(&kg, &communities, &analysis, &out_dir)
            .expect("Failed to write graph.json");
    }

    // HTML
    if formats.contains(&"html".to_string()) {
        export::export_html(&kg, &communities, &analysis, &out_dir)
            .expect("Failed to write graph.html");
    }

    // Report
    if formats.contains(&"md".to_string()) {
        let report_content = report::render_report(&analysis, &communities);
        std::fs::write(out_dir.join("GRAPH_REPORT.md"), &report_content)
            .expect("Failed to write GRAPH_REPORT.md");
    }

    // Save cache manifest
    if !cli.no_cache {
        cache::save_manifest(&cache_dir, &manifest);
    }

    // ── Done ────────────────────────────────────────────────────────
    let total_time = start.elapsed();
    println!();
    println!(
        "  {} {}",
        "✓".green().bold(),
        "Output:".bold()
    );
    if formats.contains(&"json".to_string()) {
        println!("    {} graph.json         — queryable graph data", "→".dimmed());
    }
    if formats.contains(&"html".to_string()) {
        println!("    {} graph.html         — interactive visualization", "→".dimmed());
    }
    if formats.contains(&"md".to_string()) {
        println!("    {} GRAPH_REPORT.md    — god nodes, communities, questions", "→".dimmed());
    }
    println!();
    println!(
        "  {} in {:.2}s",
        "Done".green().bold(),
        total_time.as_secs_f64()
    );
    println!();
}
