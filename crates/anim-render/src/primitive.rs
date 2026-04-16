//! Procedural mesh generation — cube, sphere, plane, cylinder.
//!
//! Generates vertex/index data for basic geometric primitives,
//! usable both for debug visualization and scene objects.

use glam::{Vec3, Vec2};

/// A generated mesh with positions, normals, UVs, and indices.
pub struct PrimitiveMesh {
    pub positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub texcoords: Vec<Vec2>,
    pub indices: Vec<u32>,
}

/// Generate a unit cube centered at origin.
pub fn cube(width: f32, height: f32, depth: f32) -> PrimitiveMesh {
    let hw = width * 0.5;
    let hh = height * 0.5;
    let hd = depth * 0.5;

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut texcoords = Vec::new();
    let mut indices = Vec::new();

    // 6 faces, 4 vertices each
    let faces: &[([f32; 3], [[f32; 3]; 4])] = &[
        // +Y (top)
        ([0.0, 1.0, 0.0], [[-hw, hh, -hd], [hw, hh, -hd], [hw, hh, hd], [-hw, hh, hd]]),
        // -Y (bottom)
        ([0.0, -1.0, 0.0], [[-hw, -hh, hd], [hw, -hh, hd], [hw, -hh, -hd], [-hw, -hh, -hd]]),
        // +Z (front)
        ([0.0, 0.0, 1.0], [[-hw, -hh, hd], [hw, -hh, hd], [hw, hh, hd], [-hw, hh, hd]]),
        // -Z (back)
        ([0.0, 0.0, -1.0], [[hw, -hh, -hd], [-hw, -hh, -hd], [-hw, hh, -hd], [hw, hh, -hd]]),
        // +X (right)
        ([1.0, 0.0, 0.0], [[hw, -hh, hd], [hw, -hh, -hd], [hw, hh, -hd], [hw, hh, hd]]),
        // -X (left)
        ([-1.0, 0.0, 0.0], [[-hw, -hh, -hd], [-hw, -hh, hd], [-hw, hh, hd], [-hw, hh, -hd]]),
    ];

    let face_uvs = [
        Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0),
        Vec2::new(1.0, 1.0), Vec2::new(0.0, 1.0),
    ];

    for (normal, verts) in faces {
        let base = positions.len() as u32;
        let n = Vec3::from_array(*normal);
        for (i, v) in verts.iter().enumerate() {
            positions.push(Vec3::from_array(*v));
            normals.push(n);
            texcoords.push(face_uvs[i]);
        }
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    PrimitiveMesh { positions, normals, texcoords, indices }
}

/// Generate a UV sphere.
pub fn sphere(radius: f32, segments: u32, rings: u32) -> PrimitiveMesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut texcoords = Vec::new();
    let mut indices = Vec::new();

    for j in 0..=rings {
        let v = j as f32 / rings as f32;
        let phi = v * std::f32::consts::PI;
        for i in 0..=segments {
            let u = i as f32 / segments as f32;
            let theta = u * std::f32::consts::TAU;

            let x = theta.cos() * phi.sin();
            let y = phi.cos();
            let z = theta.sin() * phi.sin();

            let normal = Vec3::new(x, y, z);
            positions.push(normal * radius);
            normals.push(normal);
            texcoords.push(Vec2::new(u, v));
        }
    }

    for j in 0..rings {
        for i in 0..segments {
            let a = j * (segments + 1) + i;
            let b = a + segments + 1;
            indices.extend_from_slice(&[a, b, a + 1]);
            indices.extend_from_slice(&[b, b + 1, a + 1]);
        }
    }

    PrimitiveMesh { positions, normals, texcoords, indices }
}

/// Generate a ground plane (XZ) centered at origin.
pub fn plane(width: f32, depth: f32, subdivisions: u32) -> PrimitiveMesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut texcoords = Vec::new();
    let mut indices = Vec::new();

    let hw = width * 0.5;
    let hd = depth * 0.5;
    let divs = subdivisions + 1;

    for j in 0..=subdivisions {
        for i in 0..=subdivisions {
            let u = i as f32 / subdivisions as f32;
            let v = j as f32 / subdivisions as f32;
            positions.push(Vec3::new(-hw + u * width, 0.0, -hd + v * depth));
            normals.push(Vec3::Y);
            texcoords.push(Vec2::new(u, v));
        }
    }

    for j in 0..subdivisions {
        for i in 0..subdivisions {
            let a = j * divs + i;
            let b = a + divs;
            indices.extend_from_slice(&[a, b, a + 1]);
            indices.extend_from_slice(&[b, b + 1, a + 1]);
        }
    }

    PrimitiveMesh { positions, normals, texcoords, indices }
}

/// Generate a cylinder along Y axis.
pub fn cylinder(radius_bottom: f32, radius_top: f32, height: f32, segments: u32) -> PrimitiveMesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut texcoords = Vec::new();
    let mut indices = Vec::new();

    let hh = height * 0.5;

    // Side vertices
    for i in 0..=segments {
        let u = i as f32 / segments as f32;
        let theta = u * std::f32::consts::TAU;
        let cos = theta.cos();
        let sin = theta.sin();

        // Bottom
        positions.push(Vec3::new(cos * radius_bottom, -hh, sin * radius_bottom));
        normals.push(Vec3::new(cos, 0.0, sin).normalize());
        texcoords.push(Vec2::new(u, 0.0));

        // Top
        positions.push(Vec3::new(cos * radius_top, hh, sin * radius_top));
        normals.push(Vec3::new(cos, 0.0, sin).normalize());
        texcoords.push(Vec2::new(u, 1.0));
    }

    for i in 0..segments {
        let a = i * 2;
        let b = a + 1;
        let c = a + 2;
        let d = a + 3;
        indices.extend_from_slice(&[a, c, b]);
        indices.extend_from_slice(&[b, c, d]);
    }

    // Bottom cap
    let center_bottom = positions.len() as u32;
    positions.push(Vec3::new(0.0, -hh, 0.0));
    normals.push(-Vec3::Y);
    texcoords.push(Vec2::new(0.5, 0.5));
    for i in 0..segments {
        let theta = i as f32 / segments as f32 * std::f32::consts::TAU;
        let idx = positions.len() as u32;
        positions.push(Vec3::new(theta.cos() * radius_bottom, -hh, theta.sin() * radius_bottom));
        normals.push(-Vec3::Y);
        texcoords.push(Vec2::new(theta.cos() * 0.5 + 0.5, theta.sin() * 0.5 + 0.5));
        let next = if i + 1 < segments { idx + 1 } else { center_bottom + 1 };
        indices.extend_from_slice(&[center_bottom, next, idx]);
    }

    // Top cap
    let center_top = positions.len() as u32;
    positions.push(Vec3::new(0.0, hh, 0.0));
    normals.push(Vec3::Y);
    texcoords.push(Vec2::new(0.5, 0.5));
    for i in 0..segments {
        let theta = i as f32 / segments as f32 * std::f32::consts::TAU;
        let idx = positions.len() as u32;
        positions.push(Vec3::new(theta.cos() * radius_top, hh, theta.sin() * radius_top));
        normals.push(Vec3::Y);
        texcoords.push(Vec2::new(theta.cos() * 0.5 + 0.5, theta.sin() * 0.5 + 0.5));
        let next = if i + 1 < segments { idx + 1 } else { center_top + 1 };
        indices.extend_from_slice(&[center_top, idx, next]);
    }

    PrimitiveMesh { positions, normals, texcoords, indices }
}
