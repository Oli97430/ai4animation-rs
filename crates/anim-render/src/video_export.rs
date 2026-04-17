//! Video export — encode animation frames to MP4 or GIF.
//!
//! Provides two export paths:
//! 1. **GIF export**: Pure Rust, no dependencies — writes animated GIF directly
//! 2. **MP4 export**: Shells out to ffmpeg if available, falls back to PNG sequence
//!
//! Usage: create a `VideoEncoder`, push RGBA frames, then finalize.

use std::path::{Path, PathBuf};
use std::io::Write;

/// Video export format.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VideoFormat {
    /// Animated GIF (built-in encoder, no external deps).
    Gif,
    /// MP4 via ffmpeg (requires ffmpeg in PATH).
    Mp4,
    /// PNG sequence (always works).
    PngSequence,
}

impl Default for VideoFormat {
    fn default() -> Self {
        VideoFormat::PngSequence
    }
}

/// Configuration for video export.
#[derive(Debug, Clone)]
pub struct VideoConfig {
    pub format: VideoFormat,
    pub output_path: PathBuf,
    pub width: u32,
    pub height: u32,
    pub framerate: u32,
    /// GIF: quantize to this many colors (2-256).
    pub gif_colors: u16,
    /// GIF: loop count (0 = infinite).
    pub gif_loop: u16,
    /// MP4: CRF quality (0=lossless, 23=default, 51=worst).
    pub mp4_crf: u8,
}

impl Default for VideoConfig {
    fn default() -> Self {
        Self {
            format: VideoFormat::PngSequence,
            output_path: PathBuf::from("output"),
            width: 1280,
            height: 720,
            framerate: 30,
            gif_colors: 256,
            gif_loop: 0,
            mp4_crf: 23,
        }
    }
}

/// Result of a video export operation.
#[derive(Debug)]
pub struct ExportResult {
    pub path: PathBuf,
    pub frames_written: usize,
    pub duration_secs: f32,
    pub file_size: u64,
    pub format: VideoFormat,
}

/// Video encoder that collects frames and writes output.
pub struct VideoEncoder {
    config: VideoConfig,
    frames: Vec<Vec<u8>>, // RGBA pixel data per frame
    temp_dir: Option<PathBuf>,
}

impl VideoEncoder {
    pub fn new(config: VideoConfig) -> Self {
        Self {
            config,
            frames: Vec::new(),
            temp_dir: None,
        }
    }

    /// Push a frame of RGBA pixel data.
    /// Data must be width * height * 4 bytes.
    pub fn push_frame(&mut self, rgba_pixels: Vec<u8>) {
        debug_assert_eq!(
            rgba_pixels.len(),
            (self.config.width * self.config.height * 4) as usize,
            "Frame size mismatch"
        );
        self.frames.push(rgba_pixels);
    }

    /// Number of frames collected so far.
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    /// Finalize and write the video file.
    pub fn finalize(self) -> Result<ExportResult, String> {
        if self.frames.is_empty() {
            return Err("No frames to export".into());
        }

        match self.config.format {
            VideoFormat::Gif => self.write_gif(),
            VideoFormat::Mp4 => self.write_mp4(),
            VideoFormat::PngSequence => self.write_png_sequence(),
        }
    }

    /// Check if ffmpeg is available.
    pub fn ffmpeg_available() -> bool {
        std::process::Command::new("ffmpeg")
            .arg("-version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    // ── PNG sequence ────────────────────────────────────

    fn write_png_sequence(self) -> Result<ExportResult, String> {
        let dir = &self.config.output_path;
        std::fs::create_dir_all(dir).map_err(|e| format!("Cannot create dir: {}", e))?;

        let total = self.frames.len();
        for (i, frame) in self.frames.iter().enumerate() {
            let path = dir.join(format!("frame_{:04}.png", i));
            write_png(&path, frame, self.config.width, self.config.height)
                .map_err(|e| format!("PNG write error frame {}: {}", i, e))?;
        }

        let duration = total as f32 / self.config.framerate as f32;
        let total_size: u64 = std::fs::read_dir(dir)
            .map(|entries| entries.filter_map(|e| e.ok())
                .filter_map(|e| e.metadata().ok())
                .map(|m| m.len())
                .sum())
            .unwrap_or(0);

        Ok(ExportResult {
            path: dir.clone(),
            frames_written: total,
            duration_secs: duration,
            file_size: total_size,
            format: VideoFormat::PngSequence,
        })
    }

    // ── GIF export ──────────────────────────────────────

    fn write_gif(self) -> Result<ExportResult, String> {
        let path = self.config.output_path.with_extension("gif");

        let mut file = std::fs::File::create(&path)
            .map_err(|e| format!("Cannot create GIF: {}", e))?;

        let w = self.config.width as u16;
        let h = self.config.height as u16;
        let delay = (100.0 / self.config.framerate as f32) as u16; // centiseconds

        // GIF89a header
        file.write_all(b"GIF89a").map_err(|e| e.to_string())?;

        // Logical screen descriptor
        file.write_all(&w.to_le_bytes()).map_err(|e| e.to_string())?;
        file.write_all(&h.to_le_bytes()).map_err(|e| e.to_string())?;
        // Global color table: 256 colors, 8 bits, no sort
        file.write_all(&[0xF7, 0x00, 0x00]).map_err(|e| e.to_string())?;

        // Global color table (256 entries × 3 bytes = 768 bytes)
        // Build a simple uniform palette
        let palette = build_uniform_palette();
        file.write_all(&palette).map_err(|e| e.to_string())?;

        // Netscape looping extension
        file.write_all(&[
            0x21, 0xFF, 0x0B,                          // application extension
            b'N', b'E', b'T', b'S', b'C', b'A',       // "NETSCAPE"
            b'P', b'E', b'2', b'.', b'0',
            0x03, 0x01,                                 // sub-block
        ]).map_err(|e| e.to_string())?;
        file.write_all(&self.config.gif_loop.to_le_bytes()).map_err(|e| e.to_string())?;
        file.write_all(&[0x00]).map_err(|e| e.to_string())?; // terminator

        let total = self.frames.len();
        for frame in &self.frames {
            // Graphic control extension (delay)
            file.write_all(&[
                0x21, 0xF9, 0x04,       // GCE header
                0x00,                    // no transparency
            ]).map_err(|e| e.to_string())?;
            file.write_all(&delay.to_le_bytes()).map_err(|e| e.to_string())?;
            file.write_all(&[0x00, 0x00]).map_err(|e| e.to_string())?; // transparent color + terminator

            // Image descriptor
            file.write_all(&[0x2C]).map_err(|e| e.to_string())?;
            file.write_all(&0u16.to_le_bytes()).map_err(|e| e.to_string())?; // left
            file.write_all(&0u16.to_le_bytes()).map_err(|e| e.to_string())?; // top
            file.write_all(&w.to_le_bytes()).map_err(|e| e.to_string())?;
            file.write_all(&h.to_le_bytes()).map_err(|e| e.to_string())?;
            file.write_all(&[0x00]).map_err(|e| e.to_string())?; // no local color table

            // LZW minimum code size
            let min_code_size = 8u8;
            file.write_all(&[min_code_size]).map_err(|e| e.to_string())?;

            // Quantize frame to palette indices
            let indices = quantize_frame(frame, self.config.width, self.config.height, &palette);

            // LZW compress and write sub-blocks
            let compressed = lzw_compress(&indices, min_code_size);
            write_gif_sub_blocks(&mut file, &compressed).map_err(|e| e.to_string())?;

            // Block terminator
            file.write_all(&[0x00]).map_err(|e| e.to_string())?;
        }

        // GIF trailer
        file.write_all(&[0x3B]).map_err(|e| e.to_string())?;

        let file_size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        let duration = total as f32 / self.config.framerate as f32;

        Ok(ExportResult {
            path,
            frames_written: total,
            duration_secs: duration,
            file_size,
            format: VideoFormat::Gif,
        })
    }

    // ── MP4 via ffmpeg ──────────────────────────────────

    fn write_mp4(self) -> Result<ExportResult, String> {
        if !Self::ffmpeg_available() {
            // Capture config before consuming self
            let framerate = self.config.framerate;
            let mp4_crf = self.config.mp4_crf;
            let result = self.write_png_sequence()?;
            return Err(format!(
                "ffmpeg not found. PNG sequence saved to {:?}. Run: ffmpeg -framerate {} -i {:?}/frame_%04d.png -c:v libx264 -crf {} output.mp4",
                result.path, framerate, result.path, mp4_crf
            ));
        }

        // Write frames to temp dir
        let temp = std::env::temp_dir().join("ai4anim_export");
        std::fs::create_dir_all(&temp).map_err(|e| format!("Temp dir: {}", e))?;

        for (i, frame) in self.frames.iter().enumerate() {
            let path = temp.join(format!("frame_{:04}.png", i));
            write_png(&path, frame, self.config.width, self.config.height)
                .map_err(|e| format!("PNG write error: {}", e))?;
        }

        let output = self.config.output_path.with_extension("mp4");
        let total = self.frames.len();

        // Run ffmpeg
        let status = std::process::Command::new("ffmpeg")
            .args([
                "-y",
                "-framerate", &self.config.framerate.to_string(),
                "-i", &temp.join("frame_%04d.png").to_string_lossy(),
                "-c:v", "libx264",
                "-crf", &self.config.mp4_crf.to_string(),
                "-pix_fmt", "yuv420p",
                &output.to_string_lossy(),
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map_err(|e| format!("ffmpeg error: {}", e))?;

        // Clean up temp
        let _ = std::fs::remove_dir_all(&temp);

        if !status.success() {
            return Err("ffmpeg exited with error".into());
        }

        let file_size = std::fs::metadata(&output).map(|m| m.len()).unwrap_or(0);
        let duration = total as f32 / self.config.framerate as f32;

        Ok(ExportResult {
            path: output,
            frames_written: total,
            duration_secs: duration,
            file_size,
            format: VideoFormat::Mp4,
        })
    }
}

// ── PNG writer (minimal, no external dep) ───────────────────

fn write_png(path: &Path, rgba: &[u8], width: u32, height: u32) -> Result<(), String> {
    // Use the image crate if available, otherwise write raw
    let img = image::RgbaImage::from_raw(width, height, rgba.to_vec())
        .ok_or("Invalid image dimensions")?;
    img.save(path).map_err(|e| format!("{}", e))
}

// ── GIF helpers ─────────────────────────────────────────────

/// Build a uniform 6×6×6 RGB color palette (216 colors + 40 grays).
fn build_uniform_palette() -> Vec<u8> {
    let mut palette = Vec::with_capacity(768);
    // 216 uniform colors (6×6×6)
    for r in 0..6u8 {
        for g in 0..6u8 {
            for b in 0..6u8 {
                palette.push(r * 51);
                palette.push(g * 51);
                palette.push(b * 51);
            }
        }
    }
    // 40 additional grayscale entries
    for i in 0..40u8 {
        let v = (i as f32 * 255.0 / 39.0) as u8;
        palette.push(v);
        palette.push(v);
        palette.push(v);
    }
    palette
}

/// Quantize RGBA frame to palette indices using nearest-color matching.
fn quantize_frame(rgba: &[u8], width: u32, height: u32, _palette: &[u8]) -> Vec<u8> {
    let num_pixels = (width * height) as usize;
    let mut indices = Vec::with_capacity(num_pixels);

    for i in 0..num_pixels {
        let r = rgba[i * 4] as i32;
        let g = rgba[i * 4 + 1] as i32;
        let b = rgba[i * 4 + 2] as i32;

        // Fast quantization: map to 6×6×6 cube
        let ri = ((r + 25) / 51).min(5) as u8;
        let gi = ((g + 25) / 51).min(5) as u8;
        let bi = ((b + 25) / 51).min(5) as u8;
        let idx = ri as usize * 36 + gi as usize * 6 + bi as usize;
        indices.push(idx as u8);
    }

    indices
}

/// Simple LZW compression for GIF.
fn lzw_compress(data: &[u8], min_code_size: u8) -> Vec<u8> {
    let clear_code = 1u16 << min_code_size;
    let eoi_code = clear_code + 1;
    let mut next_code = eoi_code + 1;
    let mut code_size = min_code_size as u16 + 1;
    let max_dict = 4096u16;

    // Dictionary: maps (prefix_code, byte) → code
    let mut dict = std::collections::HashMap::new();

    let mut output_bits: Vec<u8> = Vec::new();
    let mut bit_buffer: u32 = 0;
    let mut bits_in_buffer: u32 = 0;

    let emit = |code: u16, code_size: u16, bit_buffer: &mut u32, bits_in_buffer: &mut u32, output: &mut Vec<u8>| {
        *bit_buffer |= (code as u32) << *bits_in_buffer;
        *bits_in_buffer += code_size as u32;
        while *bits_in_buffer >= 8 {
            output.push((*bit_buffer & 0xFF) as u8);
            *bit_buffer >>= 8;
            *bits_in_buffer -= 8;
        }
    };

    // Emit clear code
    emit(clear_code, code_size, &mut bit_buffer, &mut bits_in_buffer, &mut output_bits);

    if data.is_empty() {
        emit(eoi_code, code_size, &mut bit_buffer, &mut bits_in_buffer, &mut output_bits);
        if bits_in_buffer > 0 {
            output_bits.push((bit_buffer & 0xFF) as u8);
        }
        return output_bits;
    }

    let mut prefix = data[0] as u16;

    for &byte in &data[1..] {
        let key = ((prefix as u32) << 8) | byte as u32;
        if let Some(&code) = dict.get(&key) {
            prefix = code;
        } else {
            emit(prefix, code_size, &mut bit_buffer, &mut bits_in_buffer, &mut output_bits);

            if next_code < max_dict {
                dict.insert(key, next_code);
                next_code += 1;
                if next_code > (1 << code_size) && code_size < 12 {
                    code_size += 1;
                }
            } else {
                // Reset dictionary
                emit(clear_code, code_size, &mut bit_buffer, &mut bits_in_buffer, &mut output_bits);
                dict.clear();
                next_code = eoi_code + 1;
                code_size = min_code_size as u16 + 1;
            }

            prefix = byte as u16;
        }
    }

    emit(prefix, code_size, &mut bit_buffer, &mut bits_in_buffer, &mut output_bits);
    emit(eoi_code, code_size, &mut bit_buffer, &mut bits_in_buffer, &mut output_bits);

    if bits_in_buffer > 0 {
        output_bits.push((bit_buffer & 0xFF) as u8);
    }

    output_bits
}

/// Write GIF sub-blocks (max 255 bytes each).
fn write_gif_sub_blocks<W: Write>(writer: &mut W, data: &[u8]) -> Result<(), std::io::Error> {
    for chunk in data.chunks(255) {
        writer.write_all(&[chunk.len() as u8])?;
        writer.write_all(chunk)?;
    }
    Ok(())
}

// ── Tests ───────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encoder_creation() {
        let config = VideoConfig::default();
        let encoder = VideoEncoder::new(config);
        assert_eq!(encoder.frame_count(), 0);
    }

    #[test]
    fn test_push_frame() {
        let config = VideoConfig { width: 4, height: 4, ..Default::default() };
        let mut encoder = VideoEncoder::new(config);
        let frame = vec![0u8; 4 * 4 * 4]; // 4x4 RGBA
        encoder.push_frame(frame);
        assert_eq!(encoder.frame_count(), 1);
    }

    #[test]
    fn test_empty_finalize_error() {
        let config = VideoConfig::default();
        let encoder = VideoEncoder::new(config);
        assert!(encoder.finalize().is_err());
    }

    #[test]
    fn test_uniform_palette() {
        let palette = build_uniform_palette();
        assert_eq!(palette.len(), 768); // 256 × 3
    }

    #[test]
    fn test_quantize_frame() {
        let palette = build_uniform_palette();
        // 2x2 frame: red, green, blue, white
        let rgba = vec![
            255, 0, 0, 255,   // red
            0, 255, 0, 255,   // green
            0, 0, 255, 255,   // blue
            255, 255, 255, 255, // white
        ];
        let indices = quantize_frame(&rgba, 2, 2, &palette);
        assert_eq!(indices.len(), 4);
        // Red should map to (5,0,0) → index 5*36 = 180
        assert_eq!(indices[0], 180);
        // Green should map to (0,5,0) → index 5*6 = 30
        assert_eq!(indices[1], 30);
    }

    #[test]
    fn test_lzw_compress() {
        let data = vec![0u8; 100]; // 100 identical bytes
        let compressed = lzw_compress(&data, 8);
        // Compressed should be smaller than raw
        assert!(compressed.len() < data.len());
    }

    #[test]
    fn test_lzw_compress_empty() {
        let data = vec![];
        let compressed = lzw_compress(&data, 8);
        // Should contain at least clear code + EOI
        assert!(compressed.len() >= 2);
    }

    #[test]
    fn test_png_sequence_export() {
        let temp = std::env::temp_dir().join("ai4anim_test_export");
        let config = VideoConfig {
            format: VideoFormat::PngSequence,
            output_path: temp.clone(),
            width: 2,
            height: 2,
            framerate: 30,
            ..Default::default()
        };
        let mut encoder = VideoEncoder::new(config);
        encoder.push_frame(vec![128u8; 2 * 2 * 4]);
        encoder.push_frame(vec![200u8; 2 * 2 * 4]);
        let result = encoder.finalize().unwrap();
        assert_eq!(result.frames_written, 2);
        assert_eq!(result.format, VideoFormat::PngSequence);
        // Clean up
        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_gif_export() {
        let temp = std::env::temp_dir().join("ai4anim_test_gif");
        let config = VideoConfig {
            format: VideoFormat::Gif,
            output_path: temp.clone(),
            width: 4,
            height: 4,
            framerate: 10,
            ..Default::default()
        };
        let mut encoder = VideoEncoder::new(config);
        // 3 frames of solid color
        encoder.push_frame(vec![255, 0, 0, 255].repeat(16)); // red
        encoder.push_frame(vec![0, 255, 0, 255].repeat(16)); // green
        encoder.push_frame(vec![0, 0, 255, 255].repeat(16)); // blue

        let result = encoder.finalize().unwrap();
        assert_eq!(result.frames_written, 3);
        assert_eq!(result.format, VideoFormat::Gif);
        assert!(result.file_size > 0);

        // Verify GIF magic bytes
        let data = std::fs::read(&result.path).unwrap();
        assert_eq!(&data[..6], b"GIF89a");

        // Clean up
        let _ = std::fs::remove_file(&result.path);
    }

    #[test]
    fn test_ffmpeg_check() {
        // Just check it doesn't panic
        let _available = VideoEncoder::ffmpeg_available();
    }
}
