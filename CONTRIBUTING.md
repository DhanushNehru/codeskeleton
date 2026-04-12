# Contributing to codeskeleton

Thank you for your interest in contributing to codeskeleton! This guide will help you get started with local development and explain the project's structure.

## 🚀 Getting Started

### Prerequisites

You'll need the Rust toolchain installed on your machine. We recommend using [rustup](https://rustup.rs/):

- **Rust Version:** 1.70 or higher
- **Cargo:** Included with the Rust toolchain

### Local Development Setup

1. **Fork and Clone** the repository:
   ```bash
   git clone https://github.com/YOUR_USERNAME/codeskeleton.git
   cd codeskeleton
   ```

2. **Build** the project:
   ```bash
   cargo build
   ```

3. **Run** on a test project (for example, codeskeleton itself):
   ```bash
   cargo run -- .
   ```

## 🏗️ Project Structure

Each stage of the analysis pipeline is a pure function in its own module:

| Module | Responsibility |
|--------|---------------|
| `detect.rs` | Directory walking and file filtering |
| `cache.rs` | SHA256 file hashing and incremental build logic |
| `languages.rs` | Per-language tree-sitter configurations and node specs |
| `extract.rs` | Generic AST extraction engine using tree-sitter |
| `graph.rs` | petgraph construction and relationship assembly |
| `cluster.rs` | Label propagation community detection |
| `analyze.rs` | Identifying god nodes and surprising connections |
| `report.rs` | Markdown report generation (`GRAPH_REPORT.md`) |
| `export.rs` | JSON and HTML visualization export |
| `types.rs` | Shared core types (Node, Edge, Confidence) |

## 🌐 Adding a New Language

Adding support for a new language involves these steps:

1. **Add the tree-sitter grammar** crate for the language to `Cargo.toml`.
2. **Add a variant** to the `SupportedLanguage` enum in `src/languages.rs`.
3. **Define the `LanguageSpec`** with AST node types (classes, functions, etc.).
4. **Map the extensions** in the `from_extension()` function in `src/languages.rs`.
5. **Implement an import extractor** in `src/extract.rs`.
6. **Add test fixtures** in `tests/fixtures/` to verify extraction.

## ✅ Quality Standards

### Running Tests
Ensure all tests pass before submitting a pull request:
```bash
cargo test
```

### Code Style
We follow standard Rust formatting and linting rules. Please run these commands before committing:

1. **Format code:**
   ```bash
   cargo fmt
   ```

2. **Check lints:**
   ```bash
   cargo clippy -- -D warnings
   ```

## 📬 Opening a Pull Request

1. Create a feature branch: `git checkout -b feat/your-feature`
2. Commit your changes: `git commit -m "feat: add your feature"`
3. Push to your fork: `git push origin feat/your-feature`
4. Open a Pull Request against the `main` branch.

Please ensure your PR description clearly explains the changes and links to any relevant issues (e.g., `Closes #123`).
