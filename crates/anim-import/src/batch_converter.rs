//! Batch converter — convert multiple animation files in parallel.
//!
//! Supports: GLB, BVH, FBX, NPZ → exports as BVH or NPZ.

use std::path::{Path, PathBuf};
use anyhow::{Result, Context};

/// Result of a single file conversion.
#[derive(Clone, Debug)]
pub struct ConvertResult {
    pub source: PathBuf,
    pub output: PathBuf,
    pub success: bool,
    pub message: String,
}

/// Batch converter configuration.
#[derive(Clone, Debug)]
pub struct BatchConfig {
    /// Output directory for converted files.
    pub output_dir: PathBuf,
    /// Output format ("bvh").
    pub output_format: String,
    /// BVH scale factor.
    pub bvh_scale: f32,
    /// Whether to preserve subdirectory structure.
    pub preserve_structure: bool,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            output_dir: PathBuf::from("converted"),
            output_format: "bvh".to_string(),
            bvh_scale: 0.01,
            preserve_structure: true,
        }
    }
}

/// Collect all animation files in a directory recursively.
pub fn collect_animation_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_recursive(dir, &mut files);
    files.sort();
    files
}

fn collect_recursive(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_recursive(&path, out);
        } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            match ext.to_lowercase().as_str() {
                "glb" | "gltf" | "bvh" | "fbx" | "npz" | "npy" => {
                    out.push(path);
                }
                _ => {}
            }
        }
    }
}

/// Convert a single file to the target format.
pub fn convert_file(
    source: &Path,
    config: &BatchConfig,
    base_dir: Option<&Path>,
) -> ConvertResult {
    let result = convert_file_inner(source, config, base_dir);
    match result {
        Ok(output) => ConvertResult {
            source: source.to_path_buf(),
            output: output.clone(),
            success: true,
            message: format!("OK → {}", output.display()),
        },
        Err(e) => ConvertResult {
            source: source.to_path_buf(),
            output: PathBuf::new(),
            success: false,
            message: format!("Erreur: {}", e),
        },
    }
}

fn convert_file_inner(
    source: &Path,
    config: &BatchConfig,
    base_dir: Option<&Path>,
) -> Result<PathBuf> {
    // Load the file
    let ext = source.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let model = match ext.as_str() {
        "glb" | "gltf" => crate::GlbImporter::load(source)?,
        "bvh" => crate::BvhImporter::load(source, config.bvh_scale)?,
        "fbx" => crate::FbxImporter::load(source)?,
        "npz" => crate::NpzImporter::load(source)?,
        "npy" => crate::NpzImporter::load(source)?,
        _ => anyhow::bail!("Format non supporte: .{}", ext),
    };

    // Compute output path
    let stem = source.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");

    let output_path = if config.preserve_structure {
        if let Some(base) = base_dir {
            if let Ok(relative) = source.strip_prefix(base) {
                let mut out = config.output_dir.join(relative);
                out.set_extension(&config.output_format);
                out
            } else {
                config.output_dir.join(format!("{}.{}", stem, config.output_format))
            }
        } else {
            config.output_dir.join(format!("{}.{}", stem, config.output_format))
        }
    } else {
        config.output_dir.join(format!("{}.{}", stem, config.output_format))
    };

    // Create parent directory
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Cannot create dir: {}", parent.display()))?;
    }

    // Export
    match config.output_format.as_str() {
        "bvh" => {
            // Export as BVH (full sequence)
            let joints = &model.joint_names;
            let parents = &model.parent_indices;
            if let Some(ref anim) = model.animation_frames {
                if anim.frames.is_empty() {
                    anyhow::bail!("No animation frames");
                }
                crate::bvh_exporter::export_bvh_sequence(
                    &output_path,
                    joints,
                    parents,
                    &anim.frames,
                    anim.framerate,
                )?;
            } else {
                anyhow::bail!("No animation data to export");
            }
        }
        "npz" => {
            // Export as NPZ
            if let Some(ref anim) = model.animation_frames {
                crate::npz_exporter::export_npz(
                    &output_path,
                    &model.joint_names,
                    &model.parent_indices,
                    &anim.frames,
                    anim.framerate,
                )?;
            } else {
                anyhow::bail!("No animation data to export");
            }
        }
        _ => anyhow::bail!("Output format not supported: {}", config.output_format),
    }

    Ok(output_path)
}

/// Convert all files in a directory. Returns results for each file.
pub fn convert_directory(
    input_dir: &Path,
    config: &BatchConfig,
) -> Vec<ConvertResult> {
    let files = collect_animation_files(input_dir);
    files.iter()
        .map(|f| convert_file(f, config, Some(input_dir)))
        .collect()
}
