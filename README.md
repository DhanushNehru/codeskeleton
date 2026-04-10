# codeskeleton

Reveal the skeleton of your codebase — turn any folder of code into a queryable knowledge graph. Single binary, zero runtime dependencies, blazing fast.

```
codeskeleton .
```

```
codeskeleton-out/
├── graph.html         interactive graph — click nodes, search, filter by community
├── GRAPH_REPORT.md    god nodes, surprising connections, suggested questions
├── graph.json         persistent graph — query later without re-reading
└── cache/             SHA256 cache — re-runs only process changed files
```

## Install

**Requires:** [Rust](https://rustup.rs/) 1.70+

```bash
cargo install codeskeleton
```

Or build from source:

```bash
git clone https://github.com/DhanushNehru/codeskeleton.git
cd codeskeleton
cargo build --release
./target/release/codeskeleton .
```

## Usage

```bash
codeskeleton .              # analyze current directory
codeskeleton ./src          # analyze a specific folder
codeskeleton . --no-cache   # force full re-extraction
```

Add a `.cographignore` file to exclude folders:

```
# .cographignore
vendor/
node_modules/
dist/
*.generated.py
```

Same syntax as `.gitignore`. Patterns match against file paths relative to the analyzed folder.

## What You Get

**God nodes** — highest-degree concepts (what everything connects through)

**Surprising connections** — cross-community edges ranked by structural distance, with plain-English explanations

**Communities** — automatically detected clusters of related code with cohesion scores

**Suggested questions** — 4-5 questions the graph is uniquely positioned to answer

**Interactive visualization** — dark-themed vis.js graph with search, click-to-inspect, community coloring

**Incremental builds** — SHA256 file caching means re-runs only process changed files

## Supported Languages

| Language | Extensions | Extraction |
|----------|-----------|------------|
| Python | `.py` | Classes, functions, imports, calls via tree-sitter AST |
| JavaScript | `.js` `.jsx` | Classes, functions, imports, calls via tree-sitter AST |
| TypeScript | `.ts` `.tsx` | Classes, functions, imports, calls via tree-sitter AST |
| Rust | `.rs` | Structs, enums, traits, functions, use declarations via tree-sitter AST |
| Go | `.go` | Types, functions, methods, imports via tree-sitter AST |
| Java | `.java` | Classes, interfaces, methods, imports via tree-sitter AST |
| C | `.c` `.h` | Structs, functions, includes via tree-sitter AST |

## How It Works

codeskeleton runs a deterministic AST pass using tree-sitter. No LLM needed — pure structural extraction:

1. **Detect** — walks the directory tree respecting `.gitignore` and `.cographignore`
2. **Cache** — SHA256 hashes each file, skips unchanged files from previous runs
3. **Extract** — tree-sitter parses each file in parallel (Rayon), extracts classes/structs, functions/methods, imports, and call sites
4. **Build** — assembles all extractions into a petgraph knowledge graph
5. **Cluster** — label propagation community detection groups related nodes
6. **Analyze** — identifies god nodes (highest degree), surprising cross-community connections, generates questions
7. **Export** — writes graph.json, graph.html (vis.js), and GRAPH_REPORT.md

Every relationship is tagged **EXTRACTED** (found directly in source) or **INFERRED** (call-graph second pass). You always know what was found vs guessed.

## Architecture

```
detect → cache-check → extract (parallel) → build_graph → cluster → analyze → report → export
```

Each stage is a pure function in its own module. No shared mutable state, no side effects outside `codeskeleton-out/`.

| Module | Responsibility |
|--------|---------------|
| `detect.rs` | Directory walk, file filtering |
| `cache.rs` | SHA256 file caching |
| `languages.rs` | Per-language tree-sitter configs |
| `extract.rs` | Generic AST extraction engine |
| `graph.rs` | petgraph construction |
| `cluster.rs` | Label propagation community detection |
| `analyze.rs` | God nodes, surprising connections |
| `report.rs` | GRAPH_REPORT.md generation |
| `export.rs` | JSON + HTML visualization |
| `types.rs` | Shared types (Node, Edge, Confidence) |

## Performance

codeskeleton is written in Rust for maximum performance:

- **Parallel extraction** — Rayon processes all files across all CPU cores
- **Zero-copy parsing** — tree-sitter operates on raw bytes, no string allocation
- **Incremental builds** — SHA256 caching means only changed files are re-extracted
- **Single binary** — no Python, no Node.js, no runtime dependencies
- **Native speed** — compiled to optimized machine code with LTO

## Contributing

**Adding a language:**

1. Add the tree-sitter grammar crate to `Cargo.toml`
2. Add a variant to `SupportedLanguage` in `languages.rs`
3. Define the `LanguageSpec` with AST node types
4. Add the extension mapping in `from_extension()`
5. Add an import extractor in `extract.rs`
6. Add test fixtures

## License

MIT
