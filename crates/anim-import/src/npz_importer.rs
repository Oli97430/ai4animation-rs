//! NPY/NPZ motion dataset importer.
//!
//! Supports:
//! - `.npy` files containing a single array of transforms [F, J, 4, 4]
//! - `.npz` files (zip archives of `.npy` arrays) with:
//!     transforms, names, parents, framerate

use std::path::Path;
use std::io::Cursor;
use anyhow::{Context, Result, bail};
use glam::Mat4;
use crate::mesh::{ImportedModel, AnimationData};

pub struct NpzImporter;

impl NpzImporter {
    /// Load a motion from an `.npy` or `.npz` file.
    pub fn load(path: &Path) -> Result<ImportedModel> {
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        match ext.as_str() {
            "npy" => Self::load_npy(path),
            "npz" => Self::load_npz(path),
            _ => bail!("Unsupported format: .{}", ext),
        }
    }

    /// Load a single `.npy` file as a transform array.
    fn load_npy(path: &Path) -> Result<ImportedModel> {
        let name = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("npy_motion")
            .to_string();

        let data = std::fs::read(path)
            .with_context(|| format!("Cannot read: {}", path.display()))?;

        let npy = npyz::NpyFile::new(&data[..])
            .map_err(|e| anyhow::anyhow!("NPY parse error: {}", e))?;

        let shape: Vec<u64> = npy.shape().to_vec();
        let (num_frames, num_joints) = parse_transform_shape(&shape, None)?;

        // Try f64 first (numpy default), fall back to f32
        let raw_f32 = read_npy_as_f32(&data)?;

        let frames = build_frames(&raw_f32, num_frames, num_joints)?;

        // Generate default names and parents
        let joint_names: Vec<String> = (0..num_joints).map(|i| format!("Joint_{}", i)).collect();
        let mut parent_indices = vec![-1i32];
        for i in 1..num_joints {
            parent_indices.push((i - 1) as i32);
        }

        log::info!("NPY: {} ({} frames, {} joints)", name, num_frames, num_joints);

        Ok(ImportedModel {
            name,
            meshes: Vec::new(),
            skin: None,
            joint_names,
            parent_indices,
            animation_frames: Some(AnimationData {
                frames,
                framerate: 30.0,
            }),
        })
    }

    /// Load an `.npz` file (ZIP of .npy files).
    fn load_npz(path: &Path) -> Result<ImportedModel> {
        let name = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("npz_motion")
            .to_string();

        let archive_bytes = std::fs::read(path)
            .with_context(|| format!("Cannot read: {}", path.display()))?;

        // Parse ZIP manually (NPZ is a standard zip file)
        let entries = parse_zip_entries(&archive_bytes)?;

        // ── Find transform array ─────────────────────────────────
        let transform_names = ["transforms.npy", "frames.npy", "data.npy", "arr_0.npy"];
        let mut raw_f32 = Vec::new();
        let mut num_frames = 0usize;
        let mut num_joints = 0usize;

        for &tname in &transform_names {
            if let Some(entry_data) = entries.get(tname) {
                let npy = npyz::NpyFile::new(&entry_data[..])
                    .map_err(|e| anyhow::anyhow!("NPY parse ({}): {}", tname, e))?;
                let shape: Vec<u64> = npy.shape().to_vec();

                raw_f32 = read_npy_as_f32(entry_data)?;
                let (f, j) = parse_transform_shape(&shape, Some(raw_f32.len()))?;
                num_frames = f;
                num_joints = j;
                break;
            }
        }

        if raw_f32.is_empty() {
            // List what arrays ARE available
            let available: Vec<&str> = entries.keys().map(|s| s.as_str()).collect();
            bail!("NPZ missing transforms array. Available: {:?}", available);
        }

        let frames = build_frames(&raw_f32, num_frames, num_joints)?;

        // ── Read joint names ─────────────────────────────────────
        let joint_names = read_npz_string_array(&entries, &["names.npy", "joint_names.npy", "bone_names.npy"])
            .unwrap_or_else(|_| {
                (0..num_joints).map(|i| format!("Joint_{}", i)).collect()
            });

        // ── Read parent indices ──────────────────────────────────
        let parent_indices = read_npz_int_array(&entries, &["parents.npy", "parent_indices.npy"])
            .unwrap_or_else(|_| {
                let mut p = vec![-1i32];
                for i in 1..num_joints { p.push((i - 1) as i32); }
                p
            });

        // ── Read framerate ───────────────────────────────────────
        let framerate = read_npz_f64_scalar(&entries, &["framerate.npy", "fps.npy"])
            .map(|v| v as f32)
            .unwrap_or(30.0);

        log::info!(
            "NPZ: {} ({} frames, {} joints, {:.1} fps)",
            name, num_frames, num_joints, framerate
        );

        Ok(ImportedModel {
            name,
            meshes: Vec::new(),
            skin: None,
            joint_names,
            parent_indices,
            animation_frames: Some(AnimationData {
                frames,
                framerate,
            }),
        })
    }
}

// ═══════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════

/// Read an npy buffer as f32 (trying f64 first, then f32).
fn read_npy_as_f32(data: &[u8]) -> Result<Vec<f32>> {
    // Try f64
    if let Ok(npy) = npyz::NpyFile::new(Cursor::new(data)) {
        if let Ok(vals) = npy.into_vec::<f64>() {
            if !vals.is_empty() {
                return Ok(vals.iter().map(|&v| v as f32).collect());
            }
        }
    }
    // Try f32
    if let Ok(npy) = npyz::NpyFile::new(Cursor::new(data)) {
        if let Ok(vals) = npy.into_vec::<f32>() {
            if !vals.is_empty() {
                return Ok(vals);
            }
        }
    }
    bail!("Cannot read npy data as f32 or f64");
}

/// Parse transform shape from numpy shape metadata.
fn parse_transform_shape(shape: &[u64], total_elements: Option<usize>) -> Result<(usize, usize)> {
    if shape.len() == 4 && shape[2] == 4 && shape[3] == 4 {
        return Ok((shape[0] as usize, shape[1] as usize));
    }
    if shape.len() == 3 && shape[2] == 16 {
        return Ok((shape[0] as usize, shape[1] as usize));
    }

    // Try to infer from total element count
    if let Some(total) = total_elements {
        if let Some(result) = infer_shape(total) {
            return Ok(result);
        }
    }

    bail!("Unsupported transform shape: {:?}", shape);
}

/// Infer (num_frames, num_joints) from total element count.
fn infer_shape(total: usize) -> Option<(usize, usize)> {
    if total % 16 != 0 { return None; }
    let total_mats = total / 16;

    for &nj in &[65, 31, 25, 24, 23, 22, 21, 20, 19, 18, 17, 15, 10, 5, 3] {
        if total_mats % nj == 0 {
            let nf = total_mats / nj;
            if nf > 0 && nf < 100_000 {
                return Some((nf, nj));
            }
        }
    }
    None
}

/// Convert row-major 4x4 (numpy) to column-major (glam).
fn row_to_col_major(row: &[f32; 16]) -> [f32; 16] {
    [
        row[0], row[4], row[8],  row[12],
        row[1], row[5], row[9],  row[13],
        row[2], row[6], row[10], row[14],
        row[3], row[7], row[11], row[15],
    ]
}

/// Build frames from flat f32 array.
fn build_frames(data: &[f32], num_frames: usize, num_joints: usize) -> Result<Vec<Vec<Mat4>>> {
    if data.len() != num_frames * num_joints * 16 {
        bail!(
            "Data size mismatch: {} vs {}x{}x16",
            data.len(), num_frames, num_joints
        );
    }
    let mut frames = Vec::with_capacity(num_frames);
    for f in 0..num_frames {
        let mut joints = Vec::with_capacity(num_joints);
        for j in 0..num_joints {
            let base = (f * num_joints + j) * 16;
            let cols: [f32; 16] = data[base..base + 16].try_into().unwrap();
            joints.push(Mat4::from_cols_array(&row_to_col_major(&cols)));
        }
        frames.push(joints);
    }
    Ok(frames)
}

// ═══════════════════════════════════════════════════════════════
// Minimal ZIP parser (NPZ = ZIP of .npy files, stored/deflated)
// ═══════════════════════════════════════════════════════════════

use std::collections::HashMap;

/// Parse a ZIP archive into a map of filename -> decompressed bytes.
/// Supports STORED (no compression) entries — the most common for numpy.
fn parse_zip_entries(data: &[u8]) -> Result<HashMap<String, Vec<u8>>> {
    let mut entries = HashMap::new();
    let mut pos = 0;

    while pos + 4 <= data.len() {
        // Local file header signature = 0x04034b50
        if data[pos..pos + 4] != [0x50, 0x4b, 0x03, 0x04] {
            break; // Not a local file header
        }

        if pos + 30 > data.len() { break; }

        let compression = u16::from_le_bytes([data[pos + 8], data[pos + 9]]);
        let compressed_size = u32::from_le_bytes([
            data[pos + 18], data[pos + 19], data[pos + 20], data[pos + 21],
        ]) as usize;
        let uncompressed_size = u32::from_le_bytes([
            data[pos + 22], data[pos + 23], data[pos + 24], data[pos + 25],
        ]) as usize;
        let name_len = u16::from_le_bytes([data[pos + 26], data[pos + 27]]) as usize;
        let extra_len = u16::from_le_bytes([data[pos + 28], data[pos + 29]]) as usize;

        let name_start = pos + 30;
        let name_end = name_start + name_len;
        if name_end > data.len() { break; }

        let filename = String::from_utf8_lossy(&data[name_start..name_end]).to_string();
        let data_start = name_end + extra_len;
        let data_end = data_start + compressed_size;
        if data_end > data.len() { break; }

        match compression {
            0 => {
                // STORED — no compression
                entries.insert(filename, data[data_start..data_end].to_vec());
            }
            8 => {
                // DEFLATE — decompress
                let decoded = flate2_decoder(&data[data_start..data_end], uncompressed_size);
                match decoded {
                    Ok(decompressed) => { entries.insert(filename, decompressed); }
                    Err(e) => {
                        log::warn!("Failed to decompress {}: {}", filename, e);
                    }
                }
            }
            _ => {
                log::warn!("Unsupported compression method {} for {}", compression, filename);
            }
        }

        pos = data_end;
    }

    if entries.is_empty() {
        bail!("No entries found in ZIP archive");
    }

    Ok(entries)
}

/// Minimal DEFLATE decompression using raw inflate.
fn flate2_decoder(compressed: &[u8], expected_size: usize) -> Result<Vec<u8>> {
    // Use miniz_oxide (already a transitive dependency via wgpu/image)
    let mut decompressed = vec![0u8; expected_size];
    let result = miniz_oxide::inflate::decompress_slice_iter_to_slice(
        &mut decompressed,
        std::iter::once(compressed),
        false, // not zlib, raw deflate
        false,
    );
    match result {
        Ok(actual_size) => {
            decompressed.truncate(actual_size);
            Ok(decompressed)
        }
        Err(e) => bail!("DEFLATE error: {:?}", e),
    }
}

// ── NPZ array helpers ──────────────────────────────────────────

fn read_npz_f64_scalar(
    entries: &HashMap<String, Vec<u8>>,
    names: &[&str],
) -> Result<f64> {
    for &name in names {
        if let Some(data) = entries.get(name) {
            if let Ok(npy) = npyz::NpyFile::new(Cursor::new(data)) {
                if let Ok(vals) = npy.into_vec::<f64>() {
                    if !vals.is_empty() { return Ok(vals[0]); }
                }
            }
            if let Ok(npy) = npyz::NpyFile::new(Cursor::new(data)) {
                if let Ok(vals) = npy.into_vec::<f32>() {
                    if !vals.is_empty() { return Ok(vals[0] as f64); }
                }
            }
        }
    }
    bail!("No scalar found");
}

fn read_npz_int_array(
    entries: &HashMap<String, Vec<u8>>,
    names: &[&str],
) -> Result<Vec<i32>> {
    for &name in names {
        if let Some(data) = entries.get(name) {
            if let Ok(npy) = npyz::NpyFile::new(Cursor::new(data)) {
                if let Ok(vals) = npy.into_vec::<i64>() {
                    if !vals.is_empty() {
                        return Ok(vals.iter().map(|&v| v as i32).collect());
                    }
                }
            }
            if let Ok(npy) = npyz::NpyFile::new(Cursor::new(data)) {
                if let Ok(vals) = npy.into_vec::<i32>() {
                    if !vals.is_empty() { return Ok(vals); }
                }
            }
        }
    }
    bail!("No int array found");
}

fn read_npz_string_array(
    entries: &HashMap<String, Vec<u8>>,
    names: &[&str],
) -> Result<Vec<String>> {
    for &name in names {
        if let Some(data) = entries.get(name) {
            if let Ok(npy) = npyz::NpyFile::new(Cursor::new(data)) {
                if let Ok(raw) = npy.into_vec::<u8>() {
                    if !raw.is_empty() {
                        return parse_numpy_strings(&raw);
                    }
                }
            }
        }
    }
    bail!("No string array found");
}

fn parse_numpy_strings(raw: &[u8]) -> Result<Vec<String>> {
    let mut strings = Vec::new();
    let mut current = String::new();
    for &b in raw {
        if b == 0 {
            if !current.is_empty() {
                strings.push(current.clone());
                current.clear();
            }
        } else if b.is_ascii_graphic() || b == b' ' {
            current.push(b as char);
        }
    }
    if !current.is_empty() {
        strings.push(current);
    }
    if strings.is_empty() {
        bail!("No strings parsed");
    }
    Ok(strings)
}
