//! GPU texture readback and PNG frame capture.
//!
//! Copies the final rendered texture to a staging buffer, maps it to CPU,
//! and saves as a PNG file. Designed for the video recorder.

use std::path::Path;

/// Captures the contents of a wgpu texture and saves it as a PNG file.
///
/// This is an async-like operation that submits GPU work, then blocks
/// until the buffer is mappable. Call this after the render pass completes.
pub fn capture_texture_to_png(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    width: u32,
    height: u32,
    output_path: &Path,
) -> Result<(), CaptureError> {
    if width == 0 || height == 0 {
        return Err(CaptureError::InvalidSize);
    }

    // Row alignment: wgpu requires rows to be aligned to 256 bytes.
    let bytes_per_pixel = 4u32; // RGBA8
    let unpadded_row_bytes = width * bytes_per_pixel;
    let padded_row_bytes = align_to(unpadded_row_bytes, wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
    let buffer_size = (padded_row_bytes * height) as u64;

    // Create staging buffer
    let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("capture_staging"),
        size: buffer_size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    // Copy texture → buffer
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("capture_encoder"),
    });
    encoder.copy_texture_to_buffer(
        wgpu::ImageCopyTexture {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::ImageCopyBuffer {
            buffer: &staging_buffer,
            layout: wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(padded_row_bytes),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
    );
    queue.submit(std::iter::once(encoder.finish()));

    // Map the buffer and read pixels
    let buffer_slice = staging_buffer.slice(..);
    let (sender, receiver) = std::sync::mpsc::channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
    });
    device.poll(wgpu::Maintain::Wait);

    receiver.recv()
        .map_err(|_| CaptureError::MapFailed)?
        .map_err(|_| CaptureError::MapFailed)?;

    // Read the data and remove row padding
    let data = buffer_slice.get_mapped_range();
    let mut pixels = Vec::with_capacity((width * height * bytes_per_pixel) as usize);
    for row in 0..height {
        let start = (row * padded_row_bytes) as usize;
        let end = start + unpadded_row_bytes as usize;
        pixels.extend_from_slice(&data[start..end]);
    }
    drop(data);
    staging_buffer.unmap();

    // Save as PNG using the image crate
    let img = image::RgbaImage::from_raw(width, height, pixels)
        .ok_or(CaptureError::ImageCreateFailed)?;
    img.save(output_path)
        .map_err(|e| CaptureError::SaveFailed(e.to_string()))?;

    Ok(())
}

/// Captures texture data to a Vec<u8> (RGBA pixels, no padding).
/// Useful when you want to process the image in memory rather than save it.
pub fn capture_texture_to_rgba(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, CaptureError> {
    if width == 0 || height == 0 {
        return Err(CaptureError::InvalidSize);
    }

    let bytes_per_pixel = 4u32;
    let unpadded_row_bytes = width * bytes_per_pixel;
    let padded_row_bytes = align_to(unpadded_row_bytes, wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
    let buffer_size = (padded_row_bytes * height) as u64;

    let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("capture_staging"),
        size: buffer_size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("capture_encoder"),
    });
    encoder.copy_texture_to_buffer(
        wgpu::ImageCopyTexture {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::ImageCopyBuffer {
            buffer: &staging_buffer,
            layout: wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(padded_row_bytes),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
    );
    queue.submit(std::iter::once(encoder.finish()));

    let buffer_slice = staging_buffer.slice(..);
    let (sender, receiver) = std::sync::mpsc::channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
    });
    device.poll(wgpu::Maintain::Wait);

    receiver.recv()
        .map_err(|_| CaptureError::MapFailed)?
        .map_err(|_| CaptureError::MapFailed)?;

    let data = buffer_slice.get_mapped_range();
    let mut pixels = Vec::with_capacity((width * height * bytes_per_pixel) as usize);
    for row in 0..height {
        let start = (row * padded_row_bytes) as usize;
        let end = start + unpadded_row_bytes as usize;
        pixels.extend_from_slice(&data[start..end]);
    }
    drop(data);
    staging_buffer.unmap();

    Ok(pixels)
}

#[derive(Debug)]
pub enum CaptureError {
    InvalidSize,
    MapFailed,
    ImageCreateFailed,
    SaveFailed(String),
}

impl std::fmt::Display for CaptureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSize => write!(f, "Invalid texture size (0x0)"),
            Self::MapFailed => write!(f, "Failed to map GPU buffer for readback"),
            Self::ImageCreateFailed => write!(f, "Failed to create image from pixel data"),
            Self::SaveFailed(e) => write!(f, "Failed to save PNG: {}", e),
        }
    }
}

impl std::error::Error for CaptureError {}

/// Align `value` up to the next multiple of `alignment`.
fn align_to(value: u32, alignment: u32) -> u32 {
    (value + alignment - 1) & !(alignment - 1)
}
