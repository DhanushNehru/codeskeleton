//! File discovery — walk directories, respect .gitignore and .cographignore.

use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

/// Code file extensions we know how to parse via tree-sitter.
const CODE_EXTENSIONS: &[&str] = &[
    "py", "js", "jsx", "ts", "tsx", "rs", "go", "java", "c", "h",
];

/// Collect all parseable source files under `root`.
///
/// Respects `.gitignore` and `.cographignore` (same syntax as .gitignore).
/// Returns a sorted list for deterministic processing order.
pub fn collect_files(root: &Path) -> Vec<PathBuf> {
    let mut builder = WalkBuilder::new(root);
    builder.standard_filters(true); // respects .gitignore
    builder.hidden(true); // skip hidden files/dirs

    // Add .cographignore if present
    let ignore_file = root.join(".cographignore");
    if ignore_file.exists() {
        builder.add_ignore(ignore_file);
    }

    let mut files: Vec<PathBuf> = builder
        .build()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().map_or(false, |ft| ft.is_file()))
        .filter(|entry| {
            entry
                .path()
                .extension()
                .and_then(|ext| ext.to_str())
                .map_or(false, |ext| CODE_EXTENSIONS.contains(&ext))
        })
        .map(|entry| entry.into_path())
        .collect();

    files.sort();
    files
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_extensions_coverage() {
        assert!(CODE_EXTENSIONS.contains(&"py"));
        assert!(CODE_EXTENSIONS.contains(&"rs"));
        assert!(CODE_EXTENSIONS.contains(&"go"));
        assert!(!CODE_EXTENSIONS.contains(&"txt"));
        assert!(!CODE_EXTENSIONS.contains(&"md"));
    }
}
