//! Dataset — loads NPZ motion files from a directory.
//!
//! Recursively discovers .npz files and provides indexed access to motions.

use std::path::{Path, PathBuf};
use crate::motion::Motion;

/// A dataset of animation files in a directory.
pub struct Dataset {
    /// Root directory.
    pub directory: PathBuf,
    /// All discovered .npz file paths.
    pub pool: Vec<PathBuf>,
    /// Filtered subset (after calling `filter`).
    pub files: Vec<PathBuf>,
    /// Map from stem name → index in `files`.
    name_to_index: std::collections::HashMap<String, usize>,
    /// Maximum files to load (0 = unlimited).
    pub max_files: usize,
}

impl Dataset {
    /// Create a dataset from a directory. Discovers all .npz files recursively.
    pub fn new(directory: &Path, max_files: usize) -> Self {
        let mut pool = Vec::new();
        collect_npz_recursive(directory, &mut pool);
        pool.sort();

        if max_files > 0 && pool.len() > max_files {
            pool.truncate(max_files);
        }

        let mut ds = Self {
            directory: directory.to_path_buf(),
            pool,
            files: Vec::new(),
            name_to_index: std::collections::HashMap::new(),
            max_files,
        };
        ds.filter(None);
        ds
    }

    /// Filter the pool by optional substring match on filename.
    pub fn filter(&mut self, pattern: Option<&str>) {
        self.files = match pattern {
            Some(pat) => {
                let lower = pat.to_lowercase();
                self.pool.iter()
                    .filter(|p| {
                        p.file_stem()
                            .and_then(|s| s.to_str())
                            .map(|s| s.to_lowercase().contains(&lower))
                            .unwrap_or(false)
                    })
                    .cloned()
                    .collect()
            }
            None => self.pool.clone(),
        };

        // Rebuild name-to-index map
        self.name_to_index.clear();
        for (i, path) in self.files.iter().enumerate() {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                self.name_to_index.insert(stem.to_string(), i);
            }
        }
    }

    /// Number of files in the filtered set.
    pub fn len(&self) -> usize {
        self.files.len()
    }

    /// Whether the filtered set is empty.
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Load a motion from the filtered set by index.
    pub fn load_motion(&self, index: usize) -> anyhow::Result<Motion> {
        let path = self.files.get(index)
            .ok_or_else(|| anyhow::anyhow!("Index {} out of range ({})", index, self.files.len()))?;

        let model = anim_import::NpzImporter::load(path)?;
        Motion::from_imported(&model)
            .ok_or_else(|| anyhow::anyhow!("No animation in {}", path.display()))
    }

    /// Look up a motion's index by name (stem without extension).
    pub fn get_index(&self, name: &str) -> Option<usize> {
        self.name_to_index.get(name).copied()
    }

    /// Get the file path for a given index.
    pub fn get_path(&self, index: usize) -> Option<&Path> {
        self.files.get(index).map(|p| p.as_path())
    }

    /// Get the name (stem) for a given index.
    pub fn get_name(&self, index: usize) -> Option<String> {
        self.files.get(index)
            .and_then(|p| p.file_stem())
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
    }

    /// Total number of discovered files (unfiltered).
    pub fn total_pool_size(&self) -> usize {
        self.pool.len()
    }
}

fn collect_npz_recursive(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_npz_recursive(&path, out);
        } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if ext.eq_ignore_ascii_case("npz") {
                out.push(path);
            }
        }
    }
}
