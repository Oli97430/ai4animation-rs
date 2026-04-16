//! Texture loading and procedural texture generation.
//!
//! Supports loading PNG, JPG, BMP, and TGA files from disk, as well as
//! generating checkerboard and UV-test-grid textures procedurally.

use std::path::Path;
use anyhow::{Context, Result};
use crate::mesh::TextureData;

/// Load a texture from a PNG, JPG, BMP, or TGA file and return it as RGBA8
/// pixel data.
pub fn load_texture(path: &Path) -> Result<TextureData> {
    let img = image::open(path)
        .with_context(|| format!("Failed to open texture: {}", path.display()))?;

    let rgba = img.to_rgba8();
    let width = rgba.width();
    let height = rgba.height();
    let pixels = rgba.into_raw();

    Ok(TextureData {
        width,
        height,
        pixels,
    })
}

/// Generate a checkerboard texture with alternating black-and-white cells.
///
/// * `size` -- width and height of the generated texture in pixels
/// * `cell_size` -- width/height of each checkerboard cell in pixels
pub fn checkerboard_texture(size: u32, cell_size: u32) -> TextureData {
    let cell_size = cell_size.max(1);
    let mut pixels = Vec::with_capacity((size * size * 4) as usize);

    for y in 0..size {
        for x in 0..size {
            let cx = x / cell_size;
            let cy = y / cell_size;
            let is_white = (cx + cy) % 2 == 0;

            let val = if is_white { 255u8 } else { 0u8 };
            pixels.push(val); // R
            pixels.push(val); // G
            pixels.push(val); // B
            pixels.push(255); // A
        }
    }

    TextureData {
        width: size,
        height: size,
        pixels,
    }
}

/// Generate a UV debug gradient texture (red = U axis, green = V axis).
///
/// The red channel increases from 0 to 255 along the horizontal axis and
/// the green channel increases from 0 to 255 along the vertical axis,
/// producing a smooth gradient useful for verifying UV mapping.
///
/// * `size` -- width and height of the generated texture in pixels
pub fn uv_test_texture(size: u32) -> TextureData {
    let size = size.max(1);
    let mut pixels = Vec::with_capacity((size * size * 4) as usize);
    let divisor = if size > 1 { size - 1 } else { 1 };

    for y in 0..size {
        for x in 0..size {
            let r = ((x as f32 / divisor as f32) * 255.0) as u8;
            let g = ((y as f32 / divisor as f32) * 255.0) as u8;
            pixels.push(r);   // R -- maps to U
            pixels.push(g);   // G -- maps to V
            pixels.push(0);   // B
            pixels.push(255); // A
        }
    }

    TextureData {
        width: size,
        height: size,
        pixels,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checkerboard_dimensions() {
        let tex = checkerboard_texture(64, 8);
        assert_eq!(tex.width, 64);
        assert_eq!(tex.height, 64);
        assert_eq!(tex.pixels.len(), (64 * 64 * 4) as usize);
    }

    #[test]
    fn test_checkerboard_pattern() {
        let tex = checkerboard_texture(16, 8);
        // Top-left pixel (0,0) is in cell (0,0) -> white
        assert_eq!(tex.pixels[0], 255); // R
        assert_eq!(tex.pixels[1], 255); // G
        assert_eq!(tex.pixels[2], 255); // B
        assert_eq!(tex.pixels[3], 255); // A

        // Pixel (8,0) is in cell (1,0) -> black
        let offset = (8 * 4) as usize;
        assert_eq!(tex.pixels[offset], 0);
        assert_eq!(tex.pixels[offset + 1], 0);
        assert_eq!(tex.pixels[offset + 2], 0);
        assert_eq!(tex.pixels[offset + 3], 255);

        // Pixel (8,8) is in cell (1,1) -> white again
        let offset = ((8 * 16 + 8) * 4) as usize;
        assert_eq!(tex.pixels[offset], 255);
    }

    #[test]
    fn test_uv_test_creates_correct_size() {
        let tex = uv_test_texture(128);
        assert_eq!(tex.width, 128);
        assert_eq!(tex.height, 128);
        assert_eq!(tex.pixels.len(), (128 * 128 * 4) as usize);
    }

    #[test]
    fn test_uv_test_gradient_corners() {
        let tex = uv_test_texture(256);
        // Top-left (0,0): R=0, G=0
        assert_eq!(tex.pixels[0], 0);
        assert_eq!(tex.pixels[1], 0);
        assert_eq!(tex.pixels[2], 0);
        assert_eq!(tex.pixels[3], 255);

        // Top-right (255,0): R=255, G=0
        let off = (255 * 4) as usize;
        assert_eq!(tex.pixels[off], 255);
        assert_eq!(tex.pixels[off + 1], 0);

        // Bottom-left (0,255): R=0, G=255
        let off = (255 * 256 * 4) as usize;
        assert_eq!(tex.pixels[off], 0);
        assert_eq!(tex.pixels[off + 1], 255);

        // Bottom-right (255,255): R=255, G=255
        let off = ((255 * 256 + 255) * 4) as usize;
        assert_eq!(tex.pixels[off], 255);
        assert_eq!(tex.pixels[off + 1], 255);
    }

    #[test]
    fn test_load_missing_file_returns_error() {
        let result = load_texture(Path::new("/nonexistent/texture.png"));
        assert!(result.is_err());
    }

    #[test]
    fn test_checkerboard_minimum_cell_size() {
        // cell_size = 0 should be clamped to 1 and not panic
        let tex = checkerboard_texture(4, 0);
        assert_eq!(tex.width, 4);
        assert_eq!(tex.pixels.len(), (4 * 4 * 4) as usize);
    }

    #[test]
    fn test_uv_test_texture_minimum_size() {
        // size = 1 is clamped to 1
        let tex = uv_test_texture(1);
        assert_eq!(tex.width, 1);
        assert_eq!(tex.pixels.len(), (1 * 1 * 4) as usize);
    }
}
