//! NPZ motion exporter.
//!
//! Exports animation data as .npz files (ZIP of .npy arrays):
//! - transforms.npy: [F, J, 4, 4] float32 transforms
//! - names.npy: joint names
//! - parents.npy: parent indices (int32)
//! - framerate.npy: scalar float64

use std::path::Path;
use std::io::Write;
use anyhow::{Result, Context};
use glam::Mat4;

/// Export motion data to an .npz file.
pub fn export_npz(
    path: &Path,
    joint_names: &[String],
    parent_indices: &[i32],
    frames: &[Vec<Mat4>],
    framerate: f32,
) -> Result<()> {
    if frames.is_empty() {
        anyhow::bail!("No frames to export");
    }

    let num_frames = frames.len();
    let num_joints = frames[0].len();

    // Create parent directory if needed
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Cannot create dir: {}", parent.display()))?;
    }

    // Build the ZIP file manually (NPZ = ZIP of .npy files, STORED)
    let file = std::fs::File::create(path)
        .with_context(|| format!("Cannot create: {}", path.display()))?;
    let mut zip = ZipWriter::new(file);

    // 1. transforms.npy — [F, J, 4, 4] float32
    let transform_data = build_transform_npy(frames, num_frames, num_joints);
    zip.add_entry("transforms.npy", &transform_data)?;

    // 2. names.npy — string array
    let names_data = build_string_npy(joint_names);
    zip.add_entry("names.npy", &names_data)?;

    // 3. parents.npy — int32 array
    let parents_data = build_i32_npy(parent_indices);
    zip.add_entry("parents.npy", &parents_data)?;

    // 4. framerate.npy — scalar float64
    let fps_data = build_f64_scalar_npy(framerate as f64);
    zip.add_entry("framerate.npy", &fps_data)?;

    zip.finish()?;

    log::info!(
        "NPZ export: {} ({} frames, {} joints, {:.1} fps)",
        path.display(), num_frames, num_joints, framerate
    );

    Ok(())
}

// ═══════════════════════════════════════════════════════════════
// NPY format builders
// ═══════════════════════════════════════════════════════════════

/// Build a .npy file for float32 4D array [F, J, 4, 4].
fn build_transform_npy(frames: &[Vec<Mat4>], num_frames: usize, num_joints: usize) -> Vec<u8> {
    // glam Mat4 is column-major; numpy expects row-major
    let shape_str = format!("({}, {}, 4, 4)", num_frames, num_joints);
    let header = npy_header("<f4", &shape_str);
    let data_size = num_frames * num_joints * 16 * 4; // 16 floats * 4 bytes

    let mut buf = Vec::with_capacity(header.len() + data_size);
    buf.extend_from_slice(&header);

    for frame in frames {
        for mat in frame {
            let cols = mat.to_cols_array();
            // Convert column-major (glam) to row-major (numpy)
            let row = col_to_row_major(&cols);
            for &v in &row {
                buf.extend_from_slice(&v.to_le_bytes());
            }
        }
    }

    buf
}

/// Build a .npy file for an int32 1D array.
fn build_i32_npy(data: &[i32]) -> Vec<u8> {
    let shape_str = format!("({},)", data.len());
    let header = npy_header("<i4", &shape_str);

    let mut buf = Vec::with_capacity(header.len() + data.len() * 4);
    buf.extend_from_slice(&header);
    for &v in data {
        buf.extend_from_slice(&v.to_le_bytes());
    }
    buf
}

/// Build a .npy file for a scalar float64.
fn build_f64_scalar_npy(value: f64) -> Vec<u8> {
    let header = npy_header("<f8", "()");
    let mut buf = Vec::with_capacity(header.len() + 8);
    buf.extend_from_slice(&header);
    buf.extend_from_slice(&value.to_le_bytes());
    buf
}

/// Build a .npy file for a string array (as fixed-width bytes).
fn build_string_npy(strings: &[String]) -> Vec<u8> {
    // Find max string length
    let max_len = strings.iter().map(|s| s.len()).max().unwrap_or(1);
    let item_size = max_len + 1; // null-terminated

    let dtype = format!("|S{}", item_size);
    let shape_str = format!("({},)", strings.len());
    let header = npy_header(&dtype, &shape_str);

    let mut buf = Vec::with_capacity(header.len() + strings.len() * item_size);
    buf.extend_from_slice(&header);
    for s in strings {
        let bytes = s.as_bytes();
        let to_write = bytes.len().min(item_size);
        buf.extend_from_slice(&bytes[..to_write]);
        // Pad with zeros
        for _ in to_write..item_size {
            buf.push(0);
        }
    }
    buf
}

/// Build an NPY v1.0 header.
fn npy_header(dtype: &str, shape: &str) -> Vec<u8> {
    let dict = format!(
        "{{'descr': '{}', 'fortran_order': False, 'shape': {}, }}",
        dtype, shape
    );

    // Header = magic + version + HEADER_LEN (2 bytes) + dict + padding to 64-byte alignment
    let magic = b"\x93NUMPY";
    let version = [1u8, 0u8];
    let prefix_len = magic.len() + version.len() + 2; // 2 for header_len field
    let total_unpadded = prefix_len + dict.len() + 1; // +1 for newline
    let padding = (64 - (total_unpadded % 64)) % 64;
    let header_data_len = dict.len() + padding + 1; // dict + padding + newline

    let mut buf = Vec::new();
    buf.extend_from_slice(magic);
    buf.extend_from_slice(&version);
    buf.extend_from_slice(&(header_data_len as u16).to_le_bytes());
    buf.extend_from_slice(dict.as_bytes());
    for _ in 0..padding {
        buf.push(b' ');
    }
    buf.push(b'\n');

    buf
}

/// Convert column-major (glam) to row-major (numpy).
fn col_to_row_major(cols: &[f32; 16]) -> [f32; 16] {
    [
        cols[0], cols[4], cols[8],  cols[12],
        cols[1], cols[5], cols[9],  cols[13],
        cols[2], cols[6], cols[10], cols[14],
        cols[3], cols[7], cols[11], cols[15],
    ]
}

// ═══════════════════════════════════════════════════════════════
// Minimal ZIP writer (STORED, no compression — standard for NPZ)
// ═══════════════════════════════════════════════════════════════

struct ZipWriter {
    writer: std::fs::File,
    entries: Vec<ZipEntry>,
    offset: u32,
}

struct ZipEntry {
    name: String,
    offset: u32,
    size: u32,
    crc: u32,
}

impl ZipWriter {
    fn new(writer: std::fs::File) -> Self {
        Self {
            writer,
            entries: Vec::new(),
            offset: 0,
        }
    }

    fn add_entry(&mut self, name: &str, data: &[u8]) -> Result<()> {
        let crc = crc32(data);
        let size = data.len() as u32;
        let name_bytes = name.as_bytes();

        // Local file header
        let header = local_file_header(name_bytes, size, crc);
        self.writer.write_all(&header)?;
        self.writer.write_all(data)?;

        self.entries.push(ZipEntry {
            name: name.to_string(),
            offset: self.offset,
            size,
            crc,
        });

        self.offset += header.len() as u32 + size;
        Ok(())
    }

    fn finish(mut self) -> Result<()> {
        let cd_offset = self.offset;
        let mut cd_size = 0u32;

        // Central directory
        for entry in &self.entries {
            let name_bytes = entry.name.as_bytes();
            let cd = central_dir_entry(name_bytes, entry.size, entry.crc, entry.offset);
            self.writer.write_all(&cd)?;
            cd_size += cd.len() as u32;
        }

        // End of central directory
        let eocd = end_of_central_dir(self.entries.len() as u16, cd_size, cd_offset);
        self.writer.write_all(&eocd)?;

        Ok(())
    }
}

fn local_file_header(name: &[u8], size: u32, crc: u32) -> Vec<u8> {
    let mut h = Vec::with_capacity(30 + name.len());
    h.extend_from_slice(&[0x50, 0x4b, 0x03, 0x04]); // signature
    h.extend_from_slice(&20u16.to_le_bytes());        // version needed
    h.extend_from_slice(&0u16.to_le_bytes());         // flags
    h.extend_from_slice(&0u16.to_le_bytes());         // compression (STORED)
    h.extend_from_slice(&0u16.to_le_bytes());         // mod time
    h.extend_from_slice(&0u16.to_le_bytes());         // mod date
    h.extend_from_slice(&crc.to_le_bytes());          // CRC-32
    h.extend_from_slice(&size.to_le_bytes());         // compressed size
    h.extend_from_slice(&size.to_le_bytes());         // uncompressed size
    h.extend_from_slice(&(name.len() as u16).to_le_bytes()); // name length
    h.extend_from_slice(&0u16.to_le_bytes());         // extra field length
    h.extend_from_slice(name);
    h
}

fn central_dir_entry(name: &[u8], size: u32, crc: u32, local_offset: u32) -> Vec<u8> {
    let mut h = Vec::with_capacity(46 + name.len());
    h.extend_from_slice(&[0x50, 0x4b, 0x01, 0x02]); // signature
    h.extend_from_slice(&20u16.to_le_bytes());        // version made by
    h.extend_from_slice(&20u16.to_le_bytes());        // version needed
    h.extend_from_slice(&0u16.to_le_bytes());         // flags
    h.extend_from_slice(&0u16.to_le_bytes());         // compression (STORED)
    h.extend_from_slice(&0u16.to_le_bytes());         // mod time
    h.extend_from_slice(&0u16.to_le_bytes());         // mod date
    h.extend_from_slice(&crc.to_le_bytes());          // CRC-32
    h.extend_from_slice(&size.to_le_bytes());         // compressed size
    h.extend_from_slice(&size.to_le_bytes());         // uncompressed size
    h.extend_from_slice(&(name.len() as u16).to_le_bytes()); // name length
    h.extend_from_slice(&0u16.to_le_bytes());         // extra field length
    h.extend_from_slice(&0u16.to_le_bytes());         // comment length
    h.extend_from_slice(&0u16.to_le_bytes());         // disk number start
    h.extend_from_slice(&0u16.to_le_bytes());         // internal file attrs
    h.extend_from_slice(&0u32.to_le_bytes());         // external file attrs
    h.extend_from_slice(&local_offset.to_le_bytes()); // relative offset
    h.extend_from_slice(name);
    h
}

fn end_of_central_dir(entry_count: u16, cd_size: u32, cd_offset: u32) -> Vec<u8> {
    let mut h = Vec::with_capacity(22);
    h.extend_from_slice(&[0x50, 0x4b, 0x05, 0x06]); // signature
    h.extend_from_slice(&0u16.to_le_bytes());         // disk number
    h.extend_from_slice(&0u16.to_le_bytes());         // disk w/ central dir
    h.extend_from_slice(&entry_count.to_le_bytes());  // entries on this disk
    h.extend_from_slice(&entry_count.to_le_bytes());  // total entries
    h.extend_from_slice(&cd_size.to_le_bytes());      // size of central dir
    h.extend_from_slice(&cd_offset.to_le_bytes());    // offset of central dir
    h.extend_from_slice(&0u16.to_le_bytes());         // comment length
    h
}

/// Simple CRC-32 (IEEE 802.3).
fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFFFFFF;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB88320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}
