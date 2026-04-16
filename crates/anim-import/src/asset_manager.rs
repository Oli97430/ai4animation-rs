//! Asset manager with path resolution, format detection, and model caching.
//!
//! Mirrors Python's AssetManager.py but adds an LRU-style cache so
//! re-loading the same file returns a cached clone.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use anyhow::{Result, anyhow};
use crate::{ImportedModel, GlbImporter, BvhImporter, NpzImporter, FbxImporter};

/// Supported file formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssetFormat {
    Glb,
    Gltf,
    Bvh,
    Npz,
    Fbx,
    Usd,
}

impl AssetFormat {
    /// Detect format from file extension.
    pub fn from_path(path: &Path) -> Option<Self> {
        let ext = path.extension()?.to_str()?.to_lowercase();
        match ext.as_str() {
            "glb" => Some(Self::Glb),
            "gltf" => Some(Self::Gltf),
            "bvh" => Some(Self::Bvh),
            "npz" => Some(Self::Npz),
            "fbx" => Some(Self::Fbx),
            "usd" | "usda" | "usdc" => Some(Self::Usd),
            _ => None,
        }
    }

    /// File filter for native file dialogs.
    pub fn all_extensions() -> &'static [&'static str] {
        &["glb", "gltf", "bvh", "npz", "fbx", "usd", "usda"]
    }
}

/// Cached model entry.
struct CacheEntry {
    /// The loaded model (we store the "skeleton" part — joints, parents, frames).
    model: ImportedModel,
    /// Number of times this asset was loaded from cache.
    hit_count: u32,
}

/// Asset manager with root path resolution and model caching.
pub struct AssetManager {
    /// Root asset directory.
    root: Option<PathBuf>,
    /// Cache: canonical path → loaded model.
    cache: HashMap<PathBuf, CacheEntry>,
    /// Maximum cache entries (evict least-used when exceeded).
    pub max_cache_size: usize,
}

impl AssetManager {
    pub fn new() -> Self {
        Self {
            root: None,
            cache: HashMap::new(),
            max_cache_size: 50,
        }
    }

    /// Set the root asset directory.
    pub fn set_root(&mut self, root: impl Into<PathBuf>) {
        self.root = Some(root.into());
    }

    /// Get the current root.
    pub fn root(&self) -> Option<&Path> {
        self.root.as_deref()
    }

    /// Resolve a path: if relative, prepend root; if absolute, use as-is.
    pub fn resolve_path(&self, path: &str) -> PathBuf {
        let p = Path::new(path);
        if p.is_absolute() {
            p.to_path_buf()
        } else if let Some(ref root) = self.root {
            root.join(path)
        } else {
            p.to_path_buf()
        }
    }

    /// Load a model from a file path.
    /// Returns a cached copy if available, otherwise loads from disk and caches.
    pub fn load(&mut self, path: &Path) -> Result<ImportedModel> {
        let canonical = path.canonicalize()
            .unwrap_or_else(|_| path.to_path_buf());

        // Check cache
        if let Some(entry) = self.cache.get_mut(&canonical) {
            entry.hit_count += 1;
            log::info!("Cache hit: {} ({}x)", path.display(), entry.hit_count);
            return Ok(clone_model(&entry.model));
        }

        // Load from disk
        let format = AssetFormat::from_path(path)
            .ok_or_else(|| anyhow!("Unknown file format: {}", path.display()))?;

        let model = load_by_format(path, format)?;

        // Cache the result
        self.ensure_cache_capacity();
        let cached = clone_model(&model);
        self.cache.insert(canonical, CacheEntry {
            model: cached,
            hit_count: 0,
        });

        Ok(model)
    }

    /// Load a model without caching (for one-off imports).
    pub fn load_uncached(path: &Path) -> Result<ImportedModel> {
        let format = AssetFormat::from_path(path)
            .ok_or_else(|| anyhow!("Unknown file format: {}", path.display()))?;
        load_by_format(path, format)
    }

    /// Clear the entire cache.
    pub fn clear_cache(&mut self) {
        self.cache.clear();
        log::info!("Asset cache cleared");
    }

    /// Remove a specific path from the cache.
    pub fn evict(&mut self, path: &Path) {
        let canonical = path.canonicalize()
            .unwrap_or_else(|_| path.to_path_buf());
        self.cache.remove(&canonical);
    }

    /// Number of cached entries.
    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }

    /// Ensure cache doesn't exceed max size (evict least-used).
    fn ensure_cache_capacity(&mut self) {
        while self.cache.len() >= self.max_cache_size {
            // Find entry with lowest hit count
            let to_remove = self.cache.iter()
                .min_by_key(|(_, e)| e.hit_count)
                .map(|(k, _)| k.clone());
            if let Some(key) = to_remove {
                self.cache.remove(&key);
            } else {
                break;
            }
        }
    }

    /// List all supported files in a directory (non-recursive).
    pub fn list_assets(dir: &Path) -> Vec<PathBuf> {
        let mut results = Vec::new();
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && AssetFormat::from_path(&path).is_some() {
                    results.push(path);
                }
            }
        }
        results.sort();
        results
    }

    /// Recursively discover all supported files under a directory.
    pub fn discover_assets(dir: &Path) -> Vec<PathBuf> {
        let mut results = Vec::new();
        discover_recursive(dir, &mut results);
        results.sort();
        results
    }
}

impl Default for AssetManager {
    fn default() -> Self {
        Self::new()
    }
}

// ── Internal helpers ──────────────────────────────────────

fn load_by_format(path: &Path, format: AssetFormat) -> Result<ImportedModel> {
    match format {
        AssetFormat::Glb | AssetFormat::Gltf => {
            GlbImporter::load(path)
        }
        AssetFormat::Bvh => {
            BvhImporter::load(path, 1.0)
        }
        AssetFormat::Npz => {
            NpzImporter::load(path)
        }
        AssetFormat::Fbx => {
            FbxImporter::load(path)
        }
        AssetFormat::Usd => {
            crate::usd_exporter::import_usd(path)
                .map_err(|e| anyhow::anyhow!("{}", e))
        }
    }
}

/// Deep clone a model (ImportedModel doesn't derive Clone due to large data).
fn clone_model(m: &ImportedModel) -> ImportedModel {
    ImportedModel {
        name: m.name.clone(),
        meshes: m.meshes.clone(),
        skin: m.skin.clone(),
        joint_names: m.joint_names.clone(),
        parent_indices: m.parent_indices.clone(),
        animation_frames: m.animation_frames.as_ref().map(|a| {
            crate::mesh::AnimationData {
                frames: a.frames.clone(),
                framerate: a.framerate,
            }
        }),
    }
}

fn discover_recursive(dir: &Path, results: &mut Vec<PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                discover_recursive(&path, results);
            } else if path.is_file() && AssetFormat::from_path(&path).is_some() {
                results.push(path);
            }
        }
    }
}
