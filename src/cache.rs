//! SHA256 file caching — only re-extract files that have changed.

use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

const MANIFEST_FILE: &str = "manifest.json";

/// Maps file paths to their SHA256 hashes.
pub type CacheManifest = HashMap<String, String>;

/// Compute SHA256 hash of a file.
fn hash_file(path: &Path) -> Option<String> {
    let data = fs::read(path).ok()?;
    let hash = Sha256::digest(&data);
    Some(format!("{:x}", hash))
}

/// Load the cache manifest from disk.
pub fn load_manifest(cache_dir: &Path) -> CacheManifest {
    let manifest_path = cache_dir.join(MANIFEST_FILE);
    if !manifest_path.exists() {
        return CacheManifest::new();
    }
    match fs::read_to_string(&manifest_path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => CacheManifest::new(),
    }
}

/// Save the cache manifest to disk.
pub fn save_manifest(cache_dir: &Path, manifest: &CacheManifest) {
    fs::create_dir_all(cache_dir).ok();
    let manifest_path = cache_dir.join(MANIFEST_FILE);
    if let Ok(json) = serde_json::to_string_pretty(manifest) {
        fs::write(manifest_path, json).ok();
    }
}

/// Check which files need re-extraction.
///
/// Returns `(files_to_extract, updated_manifest)`.
/// Files whose SHA256 hash hasn't changed since last run are skipped.
pub fn check_cache(
    files: &[PathBuf],
    cache_dir: &Path,
) -> (Vec<PathBuf>, CacheManifest) {
    let old_manifest = load_manifest(cache_dir);
    let mut new_manifest = CacheManifest::new();
    let mut to_extract = Vec::new();

    for file in files {
        let key = file.to_string_lossy().to_string();
        if let Some(hash) = hash_file(file) {
            new_manifest.insert(key.clone(), hash.clone());
            // Re-extract if hash changed or file is new
            match old_manifest.get(&key) {
                Some(old_hash) if old_hash == &hash => {
                    // File unchanged — skip
                }
                _ => {
                    to_extract.push(file.clone());
                }
            }
        }
    }

    (to_extract, new_manifest)
}
