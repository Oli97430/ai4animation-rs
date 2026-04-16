//! Procedural model generation — full humanoid with mesh, skinning, and textures.
//!
//! Generates capsule-based body segments attached to skeleton joints,
//! with proper bone weights and procedural solid-color textures.

use glam::{Mat4, Vec3, Vec2};
use crate::mesh::{ImportedMesh, ImportedSkin, ImportedModel, TextureData, AnimationData};

/// Body part color palette for procedural texturing.
#[derive(Clone)]
pub struct BodyColors {
    pub skin: [u8; 4],      // face, hands
    pub shirt: [u8; 4],     // torso
    pub pants: [u8; 4],     // legs
    pub shoes: [u8; 4],     // feet
    pub hair: [u8; 4],      // head top
}

impl Default for BodyColors {
    fn default() -> Self {
        Self {
            skin: [220, 185, 155, 255],
            shirt: [60, 90, 160, 255],
            pants: [50, 55, 65, 255],
            shoes: [40, 35, 30, 255],
            hair: [60, 40, 25, 255],
        }
    }
}

/// Configuration for procedural humanoid generation.
pub struct HumanoidConfig {
    pub height: f32,
    pub colors: BodyColors,
    /// Number of radial segments for capsules (higher = smoother).
    pub radial_segments: u32,
    /// Name for the generated model.
    pub name: String,
}

impl Default for HumanoidConfig {
    fn default() -> Self {
        Self {
            height: 1.75,
            colors: BodyColors::default(),
            radial_segments: 8,
            name: "Procedural_Humanoid".into(),
        }
    }
}

/// Bone segment definition for mesh generation.
struct BoneSegment {
    joint_a: usize,   // start joint index
    joint_b: usize,   // end joint index
    radius: f32,
    color: [u8; 4],
}

/// Generate a complete humanoid with mesh, skeleton, skinning, and optional animation.
pub fn generate_humanoid(config: &HumanoidConfig) -> ImportedModel {
    let scale = config.height / 1.75;
    let c = &config.colors;

    // ── Skeleton (19 joints, simple humanoid) ──────────────────
    let joint_defs: Vec<(&str, i32, Vec3)> = vec![
        ("Hips",           -1, Vec3::new(0.0, 1.0, 0.0)),
        ("Spine",           0, Vec3::new(0.0, 1.15, 0.0)),
        ("Spine1",          1, Vec3::new(0.0, 1.30, 0.0)),
        ("Neck",            2, Vec3::new(0.0, 1.50, 0.0)),
        ("Head",            3, Vec3::new(0.0, 1.60, 0.0)),
        ("HeadTop",         4, Vec3::new(0.0, 1.78, 0.0)),
        ("LeftShoulder",    2, Vec3::new(0.12, 1.48, 0.0)),
        ("LeftArm",         6, Vec3::new(0.28, 1.48, 0.0)),
        ("LeftForeArm",     7, Vec3::new(0.52, 1.48, 0.0)),
        ("LeftHand",        8, Vec3::new(0.72, 1.48, 0.0)),
        ("RightShoulder",   2, Vec3::new(-0.12, 1.48, 0.0)),
        ("RightArm",       10, Vec3::new(-0.28, 1.48, 0.0)),
        ("RightForeArm",   11, Vec3::new(-0.52, 1.48, 0.0)),
        ("RightHand",      12, Vec3::new(-0.72, 1.48, 0.0)),
        ("LeftUpLeg",       0, Vec3::new(0.1, 0.95, 0.0)),
        ("LeftLeg",        14, Vec3::new(0.1, 0.50, 0.0)),
        ("LeftFoot",       15, Vec3::new(0.1, 0.06, 0.05)),
        ("RightUpLeg",      0, Vec3::new(-0.1, 0.95, 0.0)),
        ("RightLeg",       17, Vec3::new(-0.1, 0.50, 0.0)),
        ("RightFoot",      18, Vec3::new(-0.1, 0.06, 0.05)),
    ];

    let joint_names: Vec<String> = joint_defs.iter().map(|(n, _, _)| n.to_string()).collect();
    let parent_indices: Vec<i32> = joint_defs.iter().map(|(_, p, _)| *p).collect();
    let positions: Vec<Vec3> = joint_defs.iter().map(|(_, _, p)| *p * scale).collect();

    // ── Body segments (pairs of joints to connect with capsules) ──
    let segments = vec![
        // Torso
        BoneSegment { joint_a: 0, joint_b: 1, radius: 0.10, color: c.shirt },
        BoneSegment { joint_a: 1, joint_b: 2, radius: 0.095, color: c.shirt },
        BoneSegment { joint_a: 2, joint_b: 3, radius: 0.085, color: c.shirt },
        // Neck
        BoneSegment { joint_a: 3, joint_b: 4, radius: 0.04, color: c.skin },
        // Head (sphere-like)
        BoneSegment { joint_a: 4, joint_b: 5, radius: 0.09, color: c.skin },
        // Left arm
        BoneSegment { joint_a: 6, joint_b: 7, radius: 0.035, color: c.shirt },
        BoneSegment { joint_a: 7, joint_b: 8, radius: 0.032, color: c.shirt },
        BoneSegment { joint_a: 8, joint_b: 9, radius: 0.028, color: c.skin },
        // Right arm
        BoneSegment { joint_a: 10, joint_b: 11, radius: 0.035, color: c.shirt },
        BoneSegment { joint_a: 11, joint_b: 12, radius: 0.032, color: c.shirt },
        BoneSegment { joint_a: 12, joint_b: 13, radius: 0.028, color: c.skin },
        // Left leg
        BoneSegment { joint_a: 14, joint_b: 15, radius: 0.055, color: c.pants },
        BoneSegment { joint_a: 15, joint_b: 16, radius: 0.045, color: c.pants },
        // Right leg
        BoneSegment { joint_a: 17, joint_b: 18, radius: 0.055, color: c.pants },
        BoneSegment { joint_a: 18, joint_b: 19, radius: 0.045, color: c.pants },
        // Feet (short, wide)
        BoneSegment { joint_a: 16, joint_b: 16, radius: 0.04, color: c.shoes },
        BoneSegment { joint_a: 19, joint_b: 19, radius: 0.04, color: c.shoes },
    ];

    // ── Generate mesh geometry ──────────────────────────────────
    let mut all_vertices = Vec::new();
    let mut all_normals = Vec::new();
    let mut all_texcoords = Vec::new();
    let mut all_indices = Vec::new();
    let mut all_bone_indices = Vec::new();
    let mut all_bone_weights = Vec::new();
    let mut texture_pixels = Vec::new();
    let tex_size = 64u32;

    // Build a simple 4x4 color texture per segment (tiled in one texture atlas)
    let segments_count = segments.len();
    let atlas_cols = 4u32;
    let atlas_rows = ((segments_count as u32 + atlas_cols - 1) / atlas_cols).max(1);
    let cell_w = tex_size / atlas_cols;
    let cell_h = tex_size / atlas_rows;

    // Initialize texture atlas
    texture_pixels.resize((tex_size * tex_size * 4) as usize, 128u8);

    for (seg_idx, seg) in segments.iter().enumerate() {
        let col = seg_idx as u32 % atlas_cols;
        let row = seg_idx as u32 / atlas_cols;

        // Fill this cell with the segment color
        for py in 0..cell_h {
            for px in 0..cell_w {
                let x = col * cell_w + px;
                let y = row * cell_h + py;
                let offset = ((y * tex_size + x) * 4) as usize;
                if offset + 3 < texture_pixels.len() {
                    texture_pixels[offset] = seg.color[0];
                    texture_pixels[offset + 1] = seg.color[1];
                    texture_pixels[offset + 2] = seg.color[2];
                    texture_pixels[offset + 3] = seg.color[3];
                }
            }
        }

        // UV center for this segment in the atlas
        let uv_cx = (col as f32 + 0.5) / atlas_cols as f32;
        let uv_cy = (row as f32 + 0.5) / atlas_rows as f32;

        // Generate capsule geometry between joint_a and joint_b
        let pos_a = positions[seg.joint_a];
        let pos_b = if seg.joint_a == seg.joint_b {
            // Foot: create a short forward-pointing capsule
            pos_a + Vec3::new(0.0, 0.0, 0.06 * scale)
        } else {
            positions[seg.joint_b]
        };

        let radius = seg.radius * scale;
        let base_vertex = all_vertices.len() as u32;

        generate_capsule(
            pos_a, pos_b, radius,
            config.radial_segments, 4,
            uv_cx, uv_cy,
            seg.joint_a as u32, seg.joint_b as u32,
            &mut all_vertices, &mut all_normals, &mut all_texcoords,
            &mut all_indices, &mut all_bone_indices, &mut all_bone_weights,
            base_vertex,
        );
    }

    // ── Build ImportedMesh ──────────────────────────────────────
    let mesh = ImportedMesh {
        vertices: all_vertices,
        normals: all_normals,
        texcoords: all_texcoords,
        indices: all_indices,
        bone_indices: all_bone_indices,
        bone_weights: all_bone_weights,
        texture: Some(TextureData {
            width: tex_size,
            height: tex_size,
            pixels: texture_pixels,
        }),
        normal_map: None,
        metallic_roughness_map: None,
        emission_map: None,
        material_index: 0,
    };

    // ── Build ImportedSkin ──────────────────────────────────────
    let inverse_bind_matrices: Vec<Mat4> = positions.iter()
        .map(|p| Mat4::from_translation(-*p))
        .collect();

    let skin = ImportedSkin {
        inverse_bind_matrices,
        joint_names: joint_names.clone(),
        joint_indices: (0..joint_names.len()).collect(),
    };

    // ── Build rest-pose animation (1 frame) ────────────────────
    let rest_transforms: Vec<Mat4> = positions.iter()
        .map(|p| Mat4::from_translation(*p))
        .collect();

    let animation = AnimationData {
        frames: vec![rest_transforms],
        framerate: 30.0,
    };

    ImportedModel {
        name: config.name.clone(),
        meshes: vec![mesh],
        skin: Some(skin),
        joint_names,
        parent_indices,
        animation_frames: Some(animation),
    }
}

/// Generate a capsule mesh between two points with proper skinning weights.
#[allow(clippy::too_many_arguments)]
fn generate_capsule(
    start: Vec3, end: Vec3, radius: f32,
    radial_segments: u32, length_segments: u32,
    uv_cx: f32, uv_cy: f32,
    bone_a: u32, bone_b: u32,
    vertices: &mut Vec<Vec3>,
    normals: &mut Vec<Vec3>,
    texcoords: &mut Vec<Vec2>,
    indices: &mut Vec<u32>,
    bone_indices: &mut Vec<[u32; 4]>,
    bone_weights: &mut Vec<[f32; 4]>,
    base: u32,
) {
    let axis = end - start;
    let length = axis.length().max(0.001);
    let dir = axis / length;

    // Build local coordinate frame
    let up = if dir.y.abs() > 0.99 { Vec3::X } else { Vec3::Y };
    let right = dir.cross(up).normalize();
    let forward = right.cross(dir).normalize();

    let total_segments = length_segments + 2; // +2 for hemisphere caps
    let mut ring_count = 0u32;

    // Generate rings along the capsule
    for s in 0..=total_segments {
        // Position along the axis + hemisphere offset
        let (center, r, blend) = if s == 0 {
            // Bottom cap center
            (start - dir * radius * 0.5, radius * 0.3, 0.0_f32)
        } else if s == total_segments {
            // Top cap center
            (end + dir * radius * 0.5, radius * 0.3, 1.0)
        } else {
            let frac = (s - 1) as f32 / (total_segments - 2) as f32;
            let center = start + axis * frac;
            (center, radius, frac)
        };

        for j in 0..radial_segments {
            let angle = j as f32 / radial_segments as f32 * std::f32::consts::TAU;
            let (sin_a, cos_a) = angle.sin_cos();

            let normal_dir = right * cos_a + forward * sin_a;
            let pos = center + normal_dir * r;

            vertices.push(pos);
            normals.push(normal_dir);
            texcoords.push(Vec2::new(uv_cx, uv_cy));

            // Skinning: blend between bone_a and bone_b based on position along axis
            let w_b = blend;
            let w_a = 1.0 - w_b;
            bone_indices.push([bone_a, bone_b, 0, 0]);
            bone_weights.push([w_a, w_b, 0.0, 0.0]);
        }
        ring_count += 1;
    }

    // Generate triangle indices (connect adjacent rings)
    for s in 0..ring_count - 1 {
        for j in 0..radial_segments {
            let curr = base + s * radial_segments + j;
            let next = base + s * radial_segments + (j + 1) % radial_segments;
            let curr_up = curr + radial_segments;
            let next_up = next + radial_segments;

            indices.push(curr);
            indices.push(next);
            indices.push(curr_up);

            indices.push(next);
            indices.push(next_up);
            indices.push(curr_up);
        }
    }
}

/// Generate a procedural run cycle animation for a simple humanoid skeleton.
/// Returns animation frames compatible with the 20-joint skeleton from generate_humanoid.
pub fn generate_run_animation(
    joint_positions: &[Vec3],
    duration_secs: f32,
    framerate: f32,
) -> AnimationData {
    let num_frames = (duration_secs * framerate).ceil() as usize;
    let num_joints = joint_positions.len();
    let mut frames = Vec::with_capacity(num_frames);

    let cycle_duration = 0.6; // one full run stride in seconds

    for f in 0..num_frames {
        let time = f as f32 / framerate;
        let phase = (time / cycle_duration) * std::f32::consts::TAU;
        let mut transforms = Vec::with_capacity(num_joints);

        for (j, rest_pos) in joint_positions.iter().enumerate() {
            let mut pos = *rest_pos;
            let name_idx = j; // matches joint order from generate_humanoid

            match name_idx {
                // Hips: bounce + forward motion + slight lateral sway
                0 => {
                    pos.y += (phase * 2.0).sin().abs() * 0.03;
                    pos.z += time * 1.5; // forward motion
                    pos.x += (phase).sin() * 0.02; // sway
                }
                // Spine chain: follow hips with dampened bounce
                1 => {
                    pos.y += (phase * 2.0).sin().abs() * 0.025;
                    pos.z += time * 1.5;
                    pos.x += (phase).sin() * 0.015;
                }
                2 => {
                    pos.y += (phase * 2.0).sin().abs() * 0.02;
                    pos.z += time * 1.5;
                    pos.x += (phase).sin() * 0.01;
                }
                // Neck
                3 => {
                    pos.y += (phase * 2.0).sin().abs() * 0.015;
                    pos.z += time * 1.5;
                }
                // Head, HeadTop: stable with forward motion
                4 | 5 => {
                    pos.y += (phase * 2.0).sin().abs() * 0.01;
                    pos.z += time * 1.5;
                }
                // Left shoulder (6)
                6 => {
                    pos.z += time * 1.5;
                    pos.y += (phase * 2.0).sin().abs() * 0.02;
                }
                // Left arm: swing opposite to right leg
                7 => {
                    pos.z += time * 1.5 + (phase).sin() * 0.12;
                    pos.y += (phase * 2.0).sin().abs() * 0.015;
                }
                // Left forearm
                8 => {
                    pos.z += time * 1.5 + (phase).sin() * 0.18;
                    pos.y += -(phase).sin().max(0.0) * 0.08;
                }
                // Left hand
                9 => {
                    pos.z += time * 1.5 + (phase).sin() * 0.22;
                    pos.y += -(phase).sin().max(0.0) * 0.1;
                }
                // Right shoulder (10)
                10 => {
                    pos.z += time * 1.5;
                    pos.y += (phase * 2.0).sin().abs() * 0.02;
                }
                // Right arm: swing opposite to left leg
                11 => {
                    pos.z += time * 1.5 - (phase).sin() * 0.12;
                    pos.y += (phase * 2.0).sin().abs() * 0.015;
                }
                // Right forearm
                12 => {
                    pos.z += time * 1.5 - (phase).sin() * 0.18;
                    pos.y += (phase).sin().max(0.0) * 0.08;
                }
                // Right hand
                13 => {
                    pos.z += time * 1.5 - (phase).sin() * 0.22;
                    pos.y += (phase).sin().max(0.0) * 0.1;
                }
                // Left upper leg: forward swing
                14 => {
                    let swing = (-phase).sin();
                    pos.z += time * 1.5 + swing * 0.1;
                    pos.y += swing.max(0.0) * 0.04;
                }
                // Left lower leg: knee bend
                15 => {
                    let swing = (-phase).sin();
                    pos.z += time * 1.5 + swing * 0.18;
                    pos.y += swing.abs() * 0.06 - (if swing < 0.0 { swing.abs() * 0.12 } else { 0.0 });
                }
                // Left foot
                16 => {
                    let swing = (-phase).sin();
                    pos.z += time * 1.5 + swing * 0.22;
                    pos.y += swing.max(0.0) * 0.1;
                }
                // Right upper leg: opposite phase
                17 => {
                    let swing = (phase).sin();
                    pos.z += time * 1.5 + swing * 0.1;
                    pos.y += swing.max(0.0) * 0.04;
                }
                // Right lower leg
                18 => {
                    let swing = (phase).sin();
                    pos.z += time * 1.5 + swing * 0.18;
                    pos.y += swing.abs() * 0.06 - (if swing < 0.0 { swing.abs() * 0.12 } else { 0.0 });
                }
                // Right foot
                19 => {
                    let swing = (phase).sin();
                    pos.z += time * 1.5 + swing * 0.22;
                    pos.y += swing.max(0.0) * 0.1;
                }
                _ => {
                    pos.z += time * 1.5;
                }
            }

            transforms.push(Mat4::from_translation(pos));
        }

        frames.push(transforms);
    }

    AnimationData {
        frames,
        framerate,
    }
}

/// Generate a procedural walk cycle animation.
pub fn generate_walk_animation(
    joint_positions: &[Vec3],
    duration_secs: f32,
    framerate: f32,
) -> AnimationData {
    let num_frames = (duration_secs * framerate).ceil() as usize;
    let num_joints = joint_positions.len();
    let mut frames = Vec::with_capacity(num_frames);

    let cycle_duration = 1.0; // one full walk stride in seconds

    for f in 0..num_frames {
        let time = f as f32 / framerate;
        let phase = (time / cycle_duration) * std::f32::consts::TAU;
        let mut transforms = Vec::with_capacity(num_joints);

        for (j, rest_pos) in joint_positions.iter().enumerate() {
            let mut pos = *rest_pos;

            match j {
                // Hips
                0 => {
                    pos.y += (phase * 2.0).sin().abs() * 0.015;
                    pos.z += time * 0.8;
                    pos.x += (phase).sin() * 0.015;
                }
                // Spine
                1..=2 => {
                    pos.y += (phase * 2.0).sin().abs() * 0.012;
                    pos.z += time * 0.8;
                    pos.x += (phase).sin() * 0.01;
                }
                // Neck, Head, HeadTop
                3..=5 => {
                    pos.z += time * 0.8;
                    pos.y += (phase * 2.0).sin().abs() * 0.008;
                }
                // Left shoulder
                6 => {
                    pos.z += time * 0.8;
                }
                // Left arm swing
                7 => {
                    pos.z += time * 0.8 + (phase).sin() * 0.06;
                }
                8 => {
                    pos.z += time * 0.8 + (phase).sin() * 0.09;
                    pos.y += -(phase).sin().max(0.0) * 0.03;
                }
                9 => {
                    pos.z += time * 0.8 + (phase).sin() * 0.11;
                    pos.y += -(phase).sin().max(0.0) * 0.04;
                }
                // Right shoulder
                10 => {
                    pos.z += time * 0.8;
                }
                // Right arm swing (opposite)
                11 => {
                    pos.z += time * 0.8 - (phase).sin() * 0.06;
                }
                12 => {
                    pos.z += time * 0.8 - (phase).sin() * 0.09;
                    pos.y += (phase).sin().max(0.0) * 0.03;
                }
                13 => {
                    pos.z += time * 0.8 - (phase).sin() * 0.11;
                    pos.y += (phase).sin().max(0.0) * 0.04;
                }
                // Left leg
                14 => {
                    let swing = (-phase).sin();
                    pos.z += time * 0.8 + swing * 0.06;
                    pos.y += swing.max(0.0) * 0.02;
                }
                15 => {
                    let swing = (-phase).sin();
                    pos.z += time * 0.8 + swing * 0.1;
                    pos.y += swing.abs() * 0.03;
                }
                16 => {
                    let swing = (-phase).sin();
                    pos.z += time * 0.8 + swing * 0.12;
                    pos.y += swing.max(0.0) * 0.04;
                }
                // Right leg (opposite)
                17 => {
                    let swing = (phase).sin();
                    pos.z += time * 0.8 + swing * 0.06;
                    pos.y += swing.max(0.0) * 0.02;
                }
                18 => {
                    let swing = (phase).sin();
                    pos.z += time * 0.8 + swing * 0.1;
                    pos.y += swing.abs() * 0.03;
                }
                19 => {
                    let swing = (phase).sin();
                    pos.z += time * 0.8 + swing * 0.12;
                    pos.y += swing.max(0.0) * 0.04;
                }
                _ => {
                    pos.z += time * 0.8;
                }
            }

            transforms.push(Mat4::from_translation(pos));
        }

        frames.push(transforms);
    }

    AnimationData { frames, framerate }
}

/// Generate a procedural idle animation (subtle breathing + weight shift).
pub fn generate_idle_animation(
    joint_positions: &[Vec3],
    duration_secs: f32,
    framerate: f32,
) -> AnimationData {
    let num_frames = (duration_secs * framerate).ceil() as usize;
    let mut frames = Vec::with_capacity(num_frames);

    for f in 0..num_frames {
        let time = f as f32 / framerate;
        let breath = (time * 1.2 * std::f32::consts::TAU).sin(); // ~1.2 Hz breathing
        let sway = (time * 0.3 * std::f32::consts::TAU).sin(); // slow weight shift

        let mut transforms = Vec::with_capacity(joint_positions.len());
        for (j, rest_pos) in joint_positions.iter().enumerate() {
            let mut pos = *rest_pos;

            match j {
                // Hips: slight sway
                0 => {
                    pos.x += sway * 0.008;
                }
                // Spine: breathing
                1 => {
                    pos.y += breath * 0.003;
                    pos.x += sway * 0.006;
                }
                2 => {
                    pos.y += breath * 0.005;
                    pos.x += sway * 0.004;
                }
                // Neck
                3 => {
                    pos.y += breath * 0.004;
                }
                // Head: subtle nod
                4 | 5 => {
                    pos.y += breath * 0.003;
                    pos.z += breath * 0.002;
                }
                // Arms: subtle sway
                7 | 8 | 9 => {
                    pos.x += sway * 0.005;
                }
                11 | 12 | 13 => {
                    pos.x += sway * 0.005;
                }
                _ => {}
            }

            transforms.push(Mat4::from_translation(pos));
        }

        frames.push(transforms);
    }

    AnimationData { frames, framerate }
}

/// Generate a procedural jump animation.
pub fn generate_jump_animation(
    joint_positions: &[Vec3],
    duration_secs: f32,
    framerate: f32,
) -> AnimationData {
    let num_frames = (duration_secs * framerate).ceil() as usize;
    let mut frames = Vec::with_capacity(num_frames);

    for f in 0..num_frames {
        let t = f as f32 / (num_frames - 1).max(1) as f32; // [0..1]

        // Jump arc: crouch → takeoff → airborne → landing → settle
        let jump_height = 0.5;
        let y_offset = if t < 0.15 {
            // Crouch phase
            -(t / 0.15) * 0.08
        } else if t < 0.4 {
            // Takeoff + airborne
            let p = (t - 0.15) / 0.25;
            -0.08 + (p * std::f32::consts::PI).sin() * (jump_height + 0.08)
        } else if t < 0.7 {
            // Peak + descend
            let p = (t - 0.4) / 0.3;
            jump_height * (1.0 - p * p)
        } else {
            // Landing + settle
            let p = (t - 0.7) / 0.3;
            -(1.0 - p) * 0.06
        };

        let mut transforms = Vec::with_capacity(joint_positions.len());
        for (j, rest_pos) in joint_positions.iter().enumerate() {
            let mut pos = *rest_pos;
            pos.y += y_offset;

            // Arms rise during jump
            if j == 7 || j == 8 || j == 9 || j == 11 || j == 12 || j == 13 {
                let arm_lift = if t < 0.2 { 0.0 } else if t < 0.5 {
                    ((t - 0.2) / 0.3).min(1.0) * 0.15
                } else if t < 0.8 {
                    0.15 * (1.0 - (t - 0.5) / 0.3)
                } else { 0.0 };
                pos.y += arm_lift;
            }

            // Legs tuck during airborne
            if j == 15 || j == 18 { // lower legs
                let tuck = if t > 0.25 && t < 0.65 {
                    let p = ((t - 0.25) / 0.4 * std::f32::consts::PI).sin();
                    p * 0.12
                } else { 0.0 };
                pos.y += tuck;
            }

            transforms.push(Mat4::from_translation(pos));
        }
        frames.push(transforms);
    }

    AnimationData { frames, framerate }
}

/// Generate a complete humanoid with a specified animation type.
/// This is the main entry point for AI-driven generation.
pub fn generate_humanoid_with_animation(
    config: &HumanoidConfig,
    anim_type: &str,
    duration_secs: f32,
) -> ImportedModel {
    let mut model = generate_humanoid(config);
    let scale = config.height / 1.75;

    // Extract rest positions from the model
    let rest_positions: Vec<Vec3> = model.animation_frames.as_ref()
        .unwrap()
        .frames[0]
        .iter()
        .map(|t| {
            let col = t.col(3);
            Vec3::new(col.x, col.y, col.z)
        })
        .collect();

    // Generate the requested animation
    let anim = match anim_type.to_lowercase().as_str() {
        "run" | "course" | "courir" => generate_run_animation(&rest_positions, duration_secs, 30.0),
        "walk" | "marche" | "marcher" => generate_walk_animation(&rest_positions, duration_secs, 30.0),
        "idle" | "repos" | "attente" => generate_idle_animation(&rest_positions, duration_secs, 30.0),
        "jump" | "saut" | "sauter" => generate_jump_animation(&rest_positions, duration_secs, 30.0),
        _ => generate_idle_animation(&rest_positions, duration_secs, 30.0),
    };

    // Scale animation if height differs from default
    let anim = if (scale - 1.0).abs() > 0.01 {
        AnimationData {
            frames: anim.frames,
            framerate: anim.framerate,
        }
    } else {
        anim
    };

    model.animation_frames = Some(anim);
    model
}

// ── Procedural creatures ──────────────────────────────────────

/// Generate a procedural spider model (8 legs, body + head).
pub fn generate_spider(height: f32) -> ImportedModel {
    let scale = height / 0.3;
    let c_body = [40, 35, 30, 255u8];
    let c_leg = [50, 45, 35, 255u8];

    // Skeleton: body_center, head, 8 legs (hip + knee + foot = 3 joints each = 24 leg joints)
    let mut joint_defs: Vec<(&str, i32, Vec3)> = vec![
        ("Body",  -1, Vec3::new(0.0, 0.15, 0.0)),
        ("Head",   0, Vec3::new(0.0, 0.18, 0.12)),
        ("Abdomen", 0, Vec3::new(0.0, 0.14, -0.15)),
    ];

    let leg_angles = [45.0_f32, 20.0, -20.0, -45.0]; // front to back
    let leg_names = [
        ("LF", 1.0), ("LMF", 1.0), ("LMB", 1.0), ("LB", 1.0),
        ("RF", -1.0), ("RMF", -1.0), ("RMB", -1.0), ("RB", -1.0),
    ];

    for (i, &(_name, side)) in leg_names.iter().enumerate() {
        let angle_idx = i % 4;
        let angle = leg_angles[angle_idx].to_radians();
        let parent = 0i32; // body
        let base_idx = joint_defs.len() as i32;

        let hip = Vec3::new(side * 0.06, 0.14, angle.sin() * 0.08);
        let knee = Vec3::new(side * 0.18, 0.22, angle.sin() * 0.15);
        let foot = Vec3::new(side * 0.25, 0.0, angle.sin() * 0.2);

        joint_defs.push((&"Hip", parent, hip));
        joint_defs.push((&"Knee", base_idx, knee));
        joint_defs.push((&"Foot", base_idx + 1, foot));
    }

    let joint_names: Vec<String> = {
        let mut names = vec!["Body".to_string(), "Head".to_string(), "Abdomen".to_string()];
        for (_i, &(name, _)) in leg_names.iter().enumerate() {
            names.push(format!("{}_Hip", name));
            names.push(format!("{}_Knee", name));
            names.push(format!("{}_Foot", name));
        }
        names
    };

    let parent_indices: Vec<i32> = joint_defs.iter().map(|(_, p, _)| *p).collect();
    let positions: Vec<Vec3> = joint_defs.iter().map(|(_, _, p)| *p * scale).collect();

    // Build segments
    let mut segments = vec![
        BoneSegment { joint_a: 0, joint_b: 1, radius: 0.05, color: c_body }, // body-head
        BoneSegment { joint_a: 0, joint_b: 2, radius: 0.07, color: c_body }, // body-abdomen
    ];
    for leg in 0..8 {
        let base = 3 + leg * 3;
        segments.push(BoneSegment { joint_a: base, joint_b: base + 1, radius: 0.015, color: c_leg });
        segments.push(BoneSegment { joint_a: base + 1, joint_b: base + 2, radius: 0.01, color: c_leg });
    }

    build_creature_model("Spider", &joint_names, &parent_indices, &positions, &segments, scale)
}

/// Generate a procedural crab model (6 legs + 2 claws).
pub fn generate_crab(height: f32) -> ImportedModel {
    let scale = height / 0.2;
    let c_body = [180, 60, 30, 255u8];
    let c_leg = [160, 55, 25, 255u8];
    let c_claw = [200, 70, 35, 255u8];

    let joint_names = vec![
        "Body", "LeftEye", "RightEye",
        "LClaw_Base", "LClaw_Mid", "LClaw_Tip",
        "RClaw_Base", "RClaw_Mid", "RClaw_Tip",
        "L1_Hip", "L1_Knee", "L1_Foot",
        "L2_Hip", "L2_Knee", "L2_Foot",
        "L3_Hip", "L3_Knee", "L3_Foot",
        "R1_Hip", "R1_Knee", "R1_Foot",
        "R2_Hip", "R2_Knee", "R2_Foot",
        "R3_Hip", "R3_Knee", "R3_Foot",
    ].iter().map(|s| s.to_string()).collect::<Vec<_>>();

    let parent_indices = vec![
        -1, 0, 0,          // body, eyes
        0, 3, 4,            // left claw
        0, 6, 7,            // right claw
        0, 9, 10,           // legs L1-L3
        0, 12, 13,
        0, 15, 16,
        0, 18, 19,          // legs R1-R3
        0, 21, 22,
        0, 24, 25,
    ];

    let positions: Vec<Vec3> = vec![
        Vec3::new(0.0, 0.08, 0.0),
        Vec3::new(0.04, 0.12, 0.06),
        Vec3::new(-0.04, 0.12, 0.06),
        // Left claw
        Vec3::new(0.08, 0.08, 0.05),
        Vec3::new(0.16, 0.10, 0.08),
        Vec3::new(0.22, 0.09, 0.06),
        // Right claw
        Vec3::new(-0.08, 0.08, 0.05),
        Vec3::new(-0.16, 0.10, 0.08),
        Vec3::new(-0.22, 0.09, 0.06),
        // Left legs
        Vec3::new(0.06, 0.06, -0.02), Vec3::new(0.14, 0.12, -0.04), Vec3::new(0.18, 0.0, -0.05),
        Vec3::new(0.06, 0.06, -0.06), Vec3::new(0.15, 0.12, -0.10), Vec3::new(0.19, 0.0, -0.12),
        Vec3::new(0.05, 0.06, -0.10), Vec3::new(0.14, 0.12, -0.16), Vec3::new(0.18, 0.0, -0.18),
        // Right legs
        Vec3::new(-0.06, 0.06, -0.02), Vec3::new(-0.14, 0.12, -0.04), Vec3::new(-0.18, 0.0, -0.05),
        Vec3::new(-0.06, 0.06, -0.06), Vec3::new(-0.15, 0.12, -0.10), Vec3::new(-0.19, 0.0, -0.12),
        Vec3::new(-0.05, 0.06, -0.10), Vec3::new(-0.14, 0.12, -0.16), Vec3::new(-0.18, 0.0, -0.18),
    ].iter().map(|p| *p * scale).collect();

    let mut segments = vec![
        BoneSegment { joint_a: 0, joint_b: 0, radius: 0.08, color: c_body }, // body
        BoneSegment { joint_a: 0, joint_b: 1, radius: 0.012, color: c_body }, // eyes
        BoneSegment { joint_a: 0, joint_b: 2, radius: 0.012, color: c_body },
    ];
    // Claws
    for base in [3usize, 6] {
        segments.push(BoneSegment { joint_a: base, joint_b: base + 1, radius: 0.02, color: c_claw });
        segments.push(BoneSegment { joint_a: base + 1, joint_b: base + 2, radius: 0.018, color: c_claw });
    }
    // Legs
    for leg in 0..6 {
        let base = 9 + leg * 3;
        segments.push(BoneSegment { joint_a: base, joint_b: base + 1, radius: 0.012, color: c_leg });
        segments.push(BoneSegment { joint_a: base + 1, joint_b: base + 2, radius: 0.008, color: c_leg });
    }

    build_creature_model("Crab", &joint_names, &parent_indices, &positions, &segments, scale)
}

/// Generate a procedural bird model.
pub fn generate_bird(height: f32) -> ImportedModel {
    let scale = height / 0.4;
    let c_body = [140, 160, 180, 255u8];
    let c_wing = [120, 140, 170, 255u8];
    let c_leg = [200, 180, 100, 255u8];
    let c_beak = [220, 180, 50, 255u8];

    let joint_names: Vec<String> = vec![
        "Body", "Neck", "Head", "Beak", "Tail",
        "LWing_Base", "LWing_Mid", "LWing_Tip",
        "RWing_Base", "RWing_Mid", "RWing_Tip",
        "LLeg_Hip", "LLeg_Knee", "LLeg_Foot",
        "RLeg_Hip", "RLeg_Knee", "RLeg_Foot",
    ].iter().map(|s| s.to_string()).collect();

    let parent_indices = vec![
        -1, 0, 1, 2, 0,        // body chain + tail
        0, 5, 6,                // left wing
        0, 8, 9,                // right wing
        0, 11, 12,              // left leg
        0, 14, 15,              // right leg
    ];

    let positions: Vec<Vec3> = vec![
        Vec3::new(0.0, 0.2, 0.0),      // body
        Vec3::new(0.0, 0.26, 0.08),    // neck
        Vec3::new(0.0, 0.32, 0.12),    // head
        Vec3::new(0.0, 0.31, 0.18),    // beak
        Vec3::new(0.0, 0.18, -0.12),   // tail
        // Left wing
        Vec3::new(0.06, 0.22, 0.0),
        Vec3::new(0.22, 0.24, -0.02),
        Vec3::new(0.35, 0.22, -0.04),
        // Right wing
        Vec3::new(-0.06, 0.22, 0.0),
        Vec3::new(-0.22, 0.24, -0.02),
        Vec3::new(-0.35, 0.22, -0.04),
        // Left leg
        Vec3::new(0.04, 0.15, -0.02),
        Vec3::new(0.04, 0.06, 0.02),
        Vec3::new(0.04, 0.0, 0.04),
        // Right leg
        Vec3::new(-0.04, 0.15, -0.02),
        Vec3::new(-0.04, 0.06, 0.02),
        Vec3::new(-0.04, 0.0, 0.04),
    ].iter().map(|p| *p * scale).collect();

    let segments = vec![
        BoneSegment { joint_a: 0, joint_b: 1, radius: 0.04, color: c_body },
        BoneSegment { joint_a: 1, joint_b: 2, radius: 0.025, color: c_body },
        BoneSegment { joint_a: 2, joint_b: 3, radius: 0.015, color: c_beak },
        BoneSegment { joint_a: 0, joint_b: 4, radius: 0.025, color: c_body },
        // Wings
        BoneSegment { joint_a: 5, joint_b: 6, radius: 0.015, color: c_wing },
        BoneSegment { joint_a: 6, joint_b: 7, radius: 0.01, color: c_wing },
        BoneSegment { joint_a: 8, joint_b: 9, radius: 0.015, color: c_wing },
        BoneSegment { joint_a: 9, joint_b: 10, radius: 0.01, color: c_wing },
        // Legs
        BoneSegment { joint_a: 11, joint_b: 12, radius: 0.01, color: c_leg },
        BoneSegment { joint_a: 12, joint_b: 13, radius: 0.008, color: c_leg },
        BoneSegment { joint_a: 14, joint_b: 15, radius: 0.01, color: c_leg },
        BoneSegment { joint_a: 15, joint_b: 16, radius: 0.008, color: c_leg },
    ];

    build_creature_model("Bird", &joint_names, &parent_indices, &positions, &segments, scale)
}

/// Generate a procedural snake model (segmented body).
pub fn generate_snake(height: f32) -> ImportedModel {
    let scale = height / 0.5;
    let c_body = [60, 100, 50, 255u8];
    let c_head = [80, 120, 60, 255u8];

    let num_segments = 16;
    let mut joint_names = Vec::new();
    let mut parent_indices = Vec::new();
    let mut positions = Vec::new();

    for i in 0..num_segments {
        joint_names.push(if i == 0 { "Head".to_string() } else { format!("Seg_{}", i) });
        parent_indices.push(if i == 0 { -1 } else { (i - 1) as i32 });
        let z = -(i as f32) * 0.035;
        let y = if i == 0 { 0.04 } else { 0.02 + (i as f32 * 0.3).sin() * 0.005 };
        positions.push(Vec3::new(0.0, y, z) * scale);
    }

    let mut segments = Vec::new();
    for i in 0..num_segments - 1 {
        let radius = if i == 0 { 0.025 } else { 0.02 - (i as f32 * 0.001).min(0.01) };
        let color = if i == 0 { c_head } else { c_body };
        segments.push(BoneSegment { joint_a: i, joint_b: i + 1, radius, color });
    }

    build_creature_model("Snake", &joint_names, &parent_indices, &positions, &segments, scale)
}

/// Generate a procedural quadruped (dog/horse-like).
pub fn generate_quadruped(height: f32) -> ImportedModel {
    let scale = height / 0.6;
    let c_body = [160, 130, 90, 255u8];
    let c_leg = [140, 115, 80, 255u8];
    let c_head = [170, 140, 100, 255u8];

    let joint_names: Vec<String> = vec![
        "Hips", "Spine", "Chest", "Neck", "Head", "Jaw", "Tail_1", "Tail_2",
        "LF_Hip", "LF_Knee", "LF_Foot",
        "RF_Hip", "RF_Knee", "RF_Foot",
        "LB_Hip", "LB_Knee", "LB_Foot",
        "RB_Hip", "RB_Knee", "RB_Foot",
    ].iter().map(|s| s.to_string()).collect();

    let parent_indices = vec![
        -1, 0, 1, 2, 3, 4, 0, 6,   // spine + tail
        2, 8, 9,                     // left front
        2, 11, 12,                   // right front
        0, 14, 15,                   // left back
        0, 17, 18,                   // right back
    ];

    let positions: Vec<Vec3> = vec![
        Vec3::new(0.0, 0.35, -0.15),    // hips
        Vec3::new(0.0, 0.38, 0.0),      // spine
        Vec3::new(0.0, 0.40, 0.12),     // chest
        Vec3::new(0.0, 0.45, 0.22),     // neck
        Vec3::new(0.0, 0.48, 0.30),     // head
        Vec3::new(0.0, 0.44, 0.36),     // jaw
        Vec3::new(0.0, 0.32, -0.22),    // tail base
        Vec3::new(0.0, 0.28, -0.32),    // tail end
        // Front left
        Vec3::new(0.06, 0.35, 0.12),
        Vec3::new(0.06, 0.18, 0.12),
        Vec3::new(0.06, 0.0, 0.14),
        // Front right
        Vec3::new(-0.06, 0.35, 0.12),
        Vec3::new(-0.06, 0.18, 0.12),
        Vec3::new(-0.06, 0.0, 0.14),
        // Back left
        Vec3::new(0.06, 0.30, -0.15),
        Vec3::new(0.06, 0.15, -0.12),
        Vec3::new(0.06, 0.0, -0.10),
        // Back right
        Vec3::new(-0.06, 0.30, -0.15),
        Vec3::new(-0.06, 0.15, -0.12),
        Vec3::new(-0.06, 0.0, -0.10),
    ].iter().map(|p| *p * scale).collect();

    let segments = vec![
        BoneSegment { joint_a: 0, joint_b: 1, radius: 0.05, color: c_body },
        BoneSegment { joint_a: 1, joint_b: 2, radius: 0.05, color: c_body },
        BoneSegment { joint_a: 2, joint_b: 3, radius: 0.035, color: c_body },
        BoneSegment { joint_a: 3, joint_b: 4, radius: 0.035, color: c_head },
        BoneSegment { joint_a: 4, joint_b: 5, radius: 0.025, color: c_head },
        BoneSegment { joint_a: 0, joint_b: 6, radius: 0.02, color: c_body },
        BoneSegment { joint_a: 6, joint_b: 7, radius: 0.012, color: c_body },
        // Front legs
        BoneSegment { joint_a: 8, joint_b: 9, radius: 0.02, color: c_leg },
        BoneSegment { joint_a: 9, joint_b: 10, radius: 0.015, color: c_leg },
        BoneSegment { joint_a: 11, joint_b: 12, radius: 0.02, color: c_leg },
        BoneSegment { joint_a: 12, joint_b: 13, radius: 0.015, color: c_leg },
        // Back legs
        BoneSegment { joint_a: 14, joint_b: 15, radius: 0.025, color: c_leg },
        BoneSegment { joint_a: 15, joint_b: 16, radius: 0.018, color: c_leg },
        BoneSegment { joint_a: 17, joint_b: 18, radius: 0.025, color: c_leg },
        BoneSegment { joint_a: 18, joint_b: 19, radius: 0.018, color: c_leg },
    ];

    build_creature_model("Quadruped", &joint_names, &parent_indices, &positions, &segments, scale)
}

/// Internal helper to build a creature model from joint + segment definitions.
fn build_creature_model(
    name: &str,
    joint_names: &[String],
    parent_indices: &[i32],
    positions: &[Vec3],
    segments: &[BoneSegment],
    scale: f32,
) -> ImportedModel {
    let mut all_vertices = Vec::new();
    let mut all_normals = Vec::new();
    let mut all_texcoords = Vec::new();
    let mut all_indices = Vec::new();
    let mut all_bone_indices = Vec::new();
    let mut all_bone_weights = Vec::new();

    let tex_size = 32u32;
    let mut texture_pixels = vec![128u8; (tex_size * tex_size * 4) as usize];

    let atlas_cols = 4u32;
    let atlas_rows = ((segments.len() as u32 + atlas_cols - 1) / atlas_cols).max(1);
    let cell_w = tex_size / atlas_cols;
    let cell_h = tex_size / atlas_rows;

    for (seg_idx, seg) in segments.iter().enumerate() {
        let col = seg_idx as u32 % atlas_cols;
        let row = seg_idx as u32 / atlas_cols;

        for py in 0..cell_h {
            for px in 0..cell_w {
                let x = col * cell_w + px;
                let y = row * cell_h + py;
                let offset = ((y * tex_size + x) * 4) as usize;
                if offset + 3 < texture_pixels.len() {
                    texture_pixels[offset] = seg.color[0];
                    texture_pixels[offset + 1] = seg.color[1];
                    texture_pixels[offset + 2] = seg.color[2];
                    texture_pixels[offset + 3] = seg.color[3];
                }
            }
        }

        let uv_cx = (col as f32 + 0.5) / atlas_cols as f32;
        let uv_cy = (row as f32 + 0.5) / atlas_rows as f32;

        let pos_a = positions[seg.joint_a];
        let pos_b = if seg.joint_a == seg.joint_b {
            pos_a + Vec3::new(0.0, 0.0, seg.radius * scale * 0.5)
        } else {
            positions[seg.joint_b]
        };
        let radius = seg.radius * scale;
        let base_vertex = all_vertices.len() as u32;

        generate_capsule(
            pos_a, pos_b, radius,
            6, 3,
            uv_cx, uv_cy,
            seg.joint_a as u32, seg.joint_b as u32,
            &mut all_vertices, &mut all_normals, &mut all_texcoords,
            &mut all_indices, &mut all_bone_indices, &mut all_bone_weights,
            base_vertex,
        );
    }

    let mesh = ImportedMesh {
        vertices: all_vertices,
        normals: all_normals,
        texcoords: all_texcoords,
        indices: all_indices,
        bone_indices: all_bone_indices,
        bone_weights: all_bone_weights,
        texture: Some(TextureData {
            width: tex_size,
            height: tex_size,
            pixels: texture_pixels,
        }),
        normal_map: None,
        metallic_roughness_map: None,
        emission_map: None,
        material_index: 0,
    };

    let inverse_bind_matrices: Vec<Mat4> = positions.iter()
        .map(|p| Mat4::from_translation(-*p))
        .collect();

    let skin = ImportedSkin {
        inverse_bind_matrices,
        joint_names: joint_names.to_vec(),
        joint_indices: (0..joint_names.len()).collect(),
    };

    let rest_transforms: Vec<Mat4> = positions.iter()
        .map(|p| Mat4::from_translation(*p))
        .collect();

    ImportedModel {
        name: name.to_string(),
        meshes: vec![mesh],
        skin: Some(skin),
        joint_names: joint_names.to_vec(),
        parent_indices: parent_indices.to_vec(),
        animation_frames: Some(AnimationData {
            frames: vec![rest_transforms],
            framerate: 30.0,
        }),
    }
}

/// Generate a creature by type name.
pub fn generate_creature(creature_type: &str, height: f32) -> ImportedModel {
    match creature_type.to_lowercase().as_str() {
        "spider" | "araignée" | "araignee" => generate_spider(height),
        "crab" | "crabe" => generate_crab(height),
        "bird" | "oiseau" => generate_bird(height),
        "snake" | "serpent" => generate_snake(height),
        "quadruped" | "quadrupede" | "dog" | "chien" | "horse" | "cheval" =>
            generate_quadruped(height),
        _ => generate_quadruped(height), // default
    }
}

// ── 3D Mesh Primitives ──────────────────────────────────────────

/// Internal helper to build an ImportedModel from raw mesh data with a single
/// "Root" joint at the origin.
fn build_primitive_model(
    name: &str,
    vertices: Vec<Vec3>,
    normals: Vec<Vec3>,
    texcoords: Vec<Vec2>,
    indices: Vec<u32>,
) -> ImportedModel {
    let n = vertices.len();
    ImportedModel {
        name: name.to_string(),
        meshes: vec![ImportedMesh {
            vertices,
            normals,
            texcoords,
            indices,
            bone_indices: vec![[0, 0, 0, 0]; n],
            bone_weights: vec![[1.0, 0.0, 0.0, 0.0]; n],
            texture: None,
            normal_map: None,
            metallic_roughness_map: None,
            emission_map: None,
            material_index: 0,
        }],
        skin: Some(ImportedSkin {
            inverse_bind_matrices: vec![Mat4::IDENTITY],
            joint_names: vec!["Root".to_string()],
            joint_indices: vec![0],
        }),
        joint_names: vec!["Root".to_string()],
        parent_indices: vec![-1],
        animation_frames: None,
    }
}

/// Generate a UV sphere with the given radius, number of longitudinal segments,
/// and number of latitudinal rings.
pub fn generate_sphere(radius: f32, segments: u32, rings: u32) -> ImportedModel {
    let segments = segments.max(3);
    let rings = rings.max(2);

    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut texcoords = Vec::new();
    let mut indices = Vec::new();

    // Generate vertices ring-by-ring from top pole to bottom pole.
    for ring in 0..=rings {
        let phi = std::f32::consts::PI * ring as f32 / rings as f32; // 0 at top, PI at bottom
        let (sin_phi, cos_phi) = phi.sin_cos();

        for seg in 0..=segments {
            let theta = std::f32::consts::TAU * seg as f32 / segments as f32;
            let (sin_theta, cos_theta) = theta.sin_cos();

            let nx = sin_phi * cos_theta;
            let ny = cos_phi;
            let nz = sin_phi * sin_theta;
            let normal = Vec3::new(nx, ny, nz);

            vertices.push(normal * radius);
            normals.push(normal);
            texcoords.push(Vec2::new(
                seg as f32 / segments as f32,
                ring as f32 / rings as f32,
            ));
        }
    }

    // Triangulate: connect adjacent rings.
    let stride = segments + 1;
    for ring in 0..rings {
        for seg in 0..segments {
            let tl = ring * stride + seg;
            let tr = ring * stride + seg + 1;
            let bl = (ring + 1) * stride + seg;
            let br = (ring + 1) * stride + seg + 1;

            // Skip degenerate triangles at the poles.
            if ring != 0 {
                indices.push(tl);
                indices.push(bl);
                indices.push(tr);
            }
            if ring != rings - 1 {
                indices.push(tr);
                indices.push(bl);
                indices.push(br);
            }
        }
    }

    build_primitive_model("Sphere", vertices, normals, texcoords, indices)
}

/// Generate an axis-aligned box (cube) with the given dimensions and per-face
/// UV coordinates spanning 0..1 on each face.
pub fn generate_cube(width: f32, height: f32, depth: f32) -> ImportedModel {
    let hw = width * 0.5;
    let hh = height * 0.5;
    let hd = depth * 0.5;

    // Each face has 4 unique vertices (so normals are flat-shaded).
    let face_data: &[([f32; 3], [f32; 3])] = &[
        // +Y (top)
        ([-hw,  hh, -hd], [0.0, 1.0, 0.0]),
        ([ hw,  hh, -hd], [0.0, 1.0, 0.0]),
        ([ hw,  hh,  hd], [0.0, 1.0, 0.0]),
        ([-hw,  hh,  hd], [0.0, 1.0, 0.0]),
        // -Y (bottom)
        ([-hw, -hh,  hd], [0.0, -1.0, 0.0]),
        ([ hw, -hh,  hd], [0.0, -1.0, 0.0]),
        ([ hw, -hh, -hd], [0.0, -1.0, 0.0]),
        ([-hw, -hh, -hd], [0.0, -1.0, 0.0]),
        // +Z (front)
        ([-hw, -hh,  hd], [0.0, 0.0, 1.0]),
        ([ hw, -hh,  hd], [0.0, 0.0, 1.0]),
        ([ hw,  hh,  hd], [0.0, 0.0, 1.0]),
        ([-hw,  hh,  hd], [0.0, 0.0, 1.0]),
        // -Z (back)
        ([ hw, -hh, -hd], [0.0, 0.0, -1.0]),
        ([-hw, -hh, -hd], [0.0, 0.0, -1.0]),
        ([-hw,  hh, -hd], [0.0, 0.0, -1.0]),
        ([ hw,  hh, -hd], [0.0, 0.0, -1.0]),
        // +X (right)
        ([ hw, -hh,  hd], [1.0, 0.0, 0.0]),
        ([ hw, -hh, -hd], [1.0, 0.0, 0.0]),
        ([ hw,  hh, -hd], [1.0, 0.0, 0.0]),
        ([ hw,  hh,  hd], [1.0, 0.0, 0.0]),
        // -X (left)
        ([-hw, -hh, -hd], [-1.0, 0.0, 0.0]),
        ([-hw, -hh,  hd], [-1.0, 0.0, 0.0]),
        ([-hw,  hh,  hd], [-1.0, 0.0, 0.0]),
        ([-hw,  hh, -hd], [-1.0, 0.0, 0.0]),
    ];

    let face_uvs = [
        Vec2::new(0.0, 0.0),
        Vec2::new(1.0, 0.0),
        Vec2::new(1.0, 1.0),
        Vec2::new(0.0, 1.0),
    ];

    let mut vertices = Vec::with_capacity(24);
    let mut normals = Vec::with_capacity(24);
    let mut texcoords = Vec::with_capacity(24);
    let mut indices = Vec::with_capacity(36);

    for (i, &(pos, norm)) in face_data.iter().enumerate() {
        vertices.push(Vec3::from(pos));
        normals.push(Vec3::from(norm));
        texcoords.push(face_uvs[i % 4]);
    }

    for face in 0..6u32 {
        let base = face * 4;
        indices.push(base);
        indices.push(base + 1);
        indices.push(base + 2);
        indices.push(base);
        indices.push(base + 2);
        indices.push(base + 3);
    }

    build_primitive_model("Cube", vertices, normals, texcoords, indices)
}

/// Generate a flat grid plane on the XZ plane centered at the origin, with the
/// given width (along X), depth (along Z), and number of subdivisions per side.
pub fn generate_plane(width: f32, depth: f32, subdivisions: u32) -> ImportedModel {
    let subdivisions = subdivisions.max(1);
    let cols = subdivisions;
    let rows = subdivisions;

    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut texcoords = Vec::new();
    let mut indices = Vec::new();

    for row in 0..=rows {
        for col in 0..=cols {
            let u = col as f32 / cols as f32;
            let v = row as f32 / rows as f32;
            let x = (u - 0.5) * width;
            let z = (v - 0.5) * depth;

            vertices.push(Vec3::new(x, 0.0, z));
            normals.push(Vec3::Y); // all face upward
            texcoords.push(Vec2::new(u, v));
        }
    }

    let stride = cols + 1;
    for row in 0..rows {
        for col in 0..cols {
            let tl = row * stride + col;
            let tr = tl + 1;
            let bl = tl + stride;
            let br = bl + 1;

            indices.push(tl);
            indices.push(bl);
            indices.push(tr);

            indices.push(tr);
            indices.push(bl);
            indices.push(br);
        }
    }

    build_primitive_model("Plane", vertices, normals, texcoords, indices)
}

/// Generate a cylinder standing along Y with the given radius, height, and
/// number of radial segments. Includes top and bottom caps.
pub fn generate_cylinder(radius: f32, height: f32, segments: u32) -> ImportedModel {
    let segments = segments.max(3);
    let half_h = height * 0.5;

    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut texcoords = Vec::new();
    let mut indices = Vec::new();

    // ── Side wall ──
    // Two rings: bottom (y = -half_h) and top (y = +half_h).
    for ring in 0..=1u32 {
        let y = if ring == 0 { -half_h } else { half_h };
        let v = ring as f32;
        for seg in 0..=segments {
            let theta = std::f32::consts::TAU * seg as f32 / segments as f32;
            let (sin_t, cos_t) = theta.sin_cos();
            vertices.push(Vec3::new(cos_t * radius, y, sin_t * radius));
            normals.push(Vec3::new(cos_t, 0.0, sin_t));
            texcoords.push(Vec2::new(seg as f32 / segments as f32, v));
        }
    }

    let stride = segments + 1;
    for seg in 0..segments {
        let bl = seg;
        let br = seg + 1;
        let tl = seg + stride;
        let tr = seg + stride + 1;

        indices.push(bl);
        indices.push(br);
        indices.push(tl);

        indices.push(br);
        indices.push(tr);
        indices.push(tl);
    }

    // ── Top cap ──
    let top_center_idx = vertices.len() as u32;
    vertices.push(Vec3::new(0.0, half_h, 0.0));
    normals.push(Vec3::Y);
    texcoords.push(Vec2::new(0.5, 0.5));

    for seg in 0..=segments {
        let theta = std::f32::consts::TAU * seg as f32 / segments as f32;
        let (sin_t, cos_t) = theta.sin_cos();
        vertices.push(Vec3::new(cos_t * radius, half_h, sin_t * radius));
        normals.push(Vec3::Y);
        texcoords.push(Vec2::new(cos_t * 0.5 + 0.5, sin_t * 0.5 + 0.5));
    }

    for seg in 0..segments {
        indices.push(top_center_idx);
        indices.push(top_center_idx + 1 + seg);
        indices.push(top_center_idx + 2 + seg);
    }

    // ── Bottom cap ──
    let bot_center_idx = vertices.len() as u32;
    vertices.push(Vec3::new(0.0, -half_h, 0.0));
    normals.push(-Vec3::Y);
    texcoords.push(Vec2::new(0.5, 0.5));

    for seg in 0..=segments {
        let theta = std::f32::consts::TAU * seg as f32 / segments as f32;
        let (sin_t, cos_t) = theta.sin_cos();
        vertices.push(Vec3::new(cos_t * radius, -half_h, sin_t * radius));
        normals.push(-Vec3::Y);
        texcoords.push(Vec2::new(cos_t * 0.5 + 0.5, sin_t * 0.5 + 0.5));
    }

    for seg in 0..segments {
        indices.push(bot_center_idx);
        indices.push(bot_center_idx + 2 + seg);
        indices.push(bot_center_idx + 1 + seg);
    }

    build_primitive_model("Cylinder", vertices, normals, texcoords, indices)
}

/// Generate a cone standing along Y with the apex at the top. Includes a bottom
/// cap disc.
pub fn generate_cone(radius: f32, height: f32, segments: u32) -> ImportedModel {
    let segments = segments.max(3);
    let half_h = height * 0.5;

    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut texcoords = Vec::new();
    let mut indices = Vec::new();

    // The slope angle for the cone side normals.
    let slope = radius / height;
    let ny = 1.0 / (1.0 + slope * slope).sqrt();
    let nr = slope * ny;

    // ── Side surface ──
    // Apex vertex is duplicated per-segment so each triangle fan slice gets its
    // own UV. Bottom ring likewise.
    for seg in 0..=segments {
        let theta = std::f32::consts::TAU * seg as f32 / segments as f32;
        let (sin_t, cos_t) = theta.sin_cos();

        let side_normal = Vec3::new(cos_t * nr, ny, sin_t * nr).normalize();

        // Bottom ring vertex
        vertices.push(Vec3::new(cos_t * radius, -half_h, sin_t * radius));
        normals.push(side_normal);
        texcoords.push(Vec2::new(seg as f32 / segments as f32, 1.0));

        // Apex vertex (duplicated per segment for unique UVs)
        vertices.push(Vec3::new(0.0, half_h, 0.0));
        normals.push(side_normal);
        let apex_u = ((seg as f32 + 0.5) / segments as f32).min(1.0);
        texcoords.push(Vec2::new(apex_u, 0.0));
    }

    for seg in 0..segments {
        let base = seg * 2; // bottom-ring of this segment
        let apex = base + 1; // apex of this segment
        let next_base = (seg + 1) * 2; // bottom-ring of next segment

        indices.push(base);
        indices.push(next_base);
        indices.push(apex);
    }

    // ── Bottom cap ──
    let bot_center_idx = vertices.len() as u32;
    vertices.push(Vec3::new(0.0, -half_h, 0.0));
    normals.push(-Vec3::Y);
    texcoords.push(Vec2::new(0.5, 0.5));

    for seg in 0..=segments {
        let theta = std::f32::consts::TAU * seg as f32 / segments as f32;
        let (sin_t, cos_t) = theta.sin_cos();
        vertices.push(Vec3::new(cos_t * radius, -half_h, sin_t * radius));
        normals.push(-Vec3::Y);
        texcoords.push(Vec2::new(cos_t * 0.5 + 0.5, sin_t * 0.5 + 0.5));
    }

    for seg in 0..segments {
        indices.push(bot_center_idx);
        indices.push(bot_center_idx + 2 + seg);
        indices.push(bot_center_idx + 1 + seg);
    }

    build_primitive_model("Cone", vertices, normals, texcoords, indices)
}

/// Generate a torus (donut) lying on the XZ plane, centered at the origin.
///
/// * `major_r` -- distance from the center of the torus to the center of the tube
/// * `minor_r` -- radius of the tube
/// * `major_seg` -- number of segments around the main ring
/// * `minor_seg` -- number of segments around the tube cross-section
pub fn generate_torus(
    major_r: f32,
    minor_r: f32,
    major_seg: u32,
    minor_seg: u32,
) -> ImportedModel {
    let major_seg = major_seg.max(3);
    let minor_seg = minor_seg.max(3);

    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut texcoords = Vec::new();
    let mut indices = Vec::new();

    for i in 0..=major_seg {
        let u = i as f32 / major_seg as f32;
        let theta = std::f32::consts::TAU * u;
        let (sin_theta, cos_theta) = theta.sin_cos();

        // Center of the tube ring at this major angle.
        let ring_center = Vec3::new(cos_theta * major_r, 0.0, sin_theta * major_r);

        for j in 0..=minor_seg {
            let v = j as f32 / minor_seg as f32;
            let phi = std::f32::consts::TAU * v;
            let (sin_phi, cos_phi) = phi.sin_cos();

            // Local offset along the tube's cross-section.
            let local_r = major_r + minor_r * cos_phi;
            let pos = Vec3::new(
                cos_theta * local_r,
                sin_phi * minor_r,
                sin_theta * local_r,
            );

            let normal = (pos - ring_center).normalize();

            vertices.push(pos);
            normals.push(normal);
            texcoords.push(Vec2::new(u, v));
        }
    }

    let stride = minor_seg + 1;
    for i in 0..major_seg {
        for j in 0..minor_seg {
            let a = i * stride + j;
            let b = i * stride + j + 1;
            let c = (i + 1) * stride + j;
            let d = (i + 1) * stride + j + 1;

            indices.push(a);
            indices.push(c);
            indices.push(b);

            indices.push(b);
            indices.push(c);
            indices.push(d);
        }
    }

    build_primitive_model("Torus", vertices, normals, texcoords, indices)
}

/// Generate a named primitive shape with a uniform `size` parameter.
///
/// Supported shapes: `"sphere"`, `"cube"`, `"plane"`, `"cylinder"`, `"cone"`,
/// `"torus"`. Unrecognised names default to a cube.
///
/// The `size` parameter is interpreted contextually:
/// * sphere -- radius
/// * cube -- side length
/// * plane -- width = depth = size
/// * cylinder -- radius = size/2, height = size
/// * cone -- radius = size/2, height = size
/// * torus -- major_r = size/2, minor_r = size/6
pub fn generate_primitive(shape: &str, size: f32) -> ImportedModel {
    match shape.to_lowercase().as_str() {
        "sphere" | "sphère" | "sphere_uv" => generate_sphere(size, 32, 16),
        "cube" | "box" | "boîte" => generate_cube(size, size, size),
        "plane" | "plan" | "grid" | "grille" => generate_plane(size, size, 1),
        "cylinder" | "cylindre" => generate_cylinder(size * 0.5, size, 32),
        "cone" | "cône" => generate_cone(size * 0.5, size, 32),
        "torus" | "tore" | "donut" => generate_torus(size * 0.5, size / 6.0, 32, 16),
        _ => generate_cube(size, size, size),
    }
}

// ── Tests for mesh primitives ────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify that every normal vector has unit length.
    fn check_normals_unit(normals: &[Vec3]) {
        for (i, n) in normals.iter().enumerate() {
            let len = n.length();
            assert!(
                (len - 1.0).abs() < 1e-4,
                "Normal {} has length {} (expected 1.0)",
                i, len
            );
        }
    }

    /// Verify that all UV coordinates lie in [0, 1].
    fn check_uvs_range(texcoords: &[Vec2]) {
        for (i, uv) in texcoords.iter().enumerate() {
            assert!(
                uv.x >= -1e-6 && uv.x <= 1.0 + 1e-6 && uv.y >= -1e-6 && uv.y <= 1.0 + 1e-6,
                "UV {} out of range: ({}, {})",
                i, uv.x, uv.y
            );
        }
    }

    /// Verify that all indices point to valid vertices.
    fn check_indices_valid(indices: &[u32], vertex_count: usize) {
        for (i, &idx) in indices.iter().enumerate() {
            assert!(
                (idx as usize) < vertex_count,
                "Index {} = {} exceeds vertex count {}",
                i, idx, vertex_count
            );
        }
    }

    /// Run all common validations on a model.
    fn validate_model(model: &ImportedModel) {
        assert!(!model.meshes.is_empty(), "Model has no meshes");
        let mesh = &model.meshes[0];

        let nv = mesh.vertices.len();
        assert!(nv > 0, "Mesh has no vertices");
        assert_eq!(mesh.normals.len(), nv, "Normal count mismatch");
        assert_eq!(mesh.texcoords.len(), nv, "Texcoord count mismatch");
        assert_eq!(mesh.bone_indices.len(), nv, "bone_indices count mismatch");
        assert_eq!(mesh.bone_weights.len(), nv, "bone_weights count mismatch");
        assert!(!mesh.indices.is_empty(), "Mesh has no indices");
        assert_eq!(mesh.indices.len() % 3, 0, "Index count not a multiple of 3");

        check_normals_unit(&mesh.normals);
        check_uvs_range(&mesh.texcoords);
        check_indices_valid(&mesh.indices, nv);

        // Verify skeleton
        assert_eq!(model.joint_names, vec!["Root".to_string()]);
        assert_eq!(model.parent_indices, vec![-1]);
        assert!(model.skin.is_some());
    }

    #[test]
    fn test_sphere() {
        let model = generate_sphere(1.0, 16, 8);
        validate_model(&model);
        assert_eq!(model.name, "Sphere");
        // Expected vertices: (rings+1) * (segments+1) = 9 * 17 = 153
        assert_eq!(model.meshes[0].vertices.len(), 9 * 17);
    }

    #[test]
    fn test_cube() {
        let model = generate_cube(1.0, 1.0, 1.0);
        validate_model(&model);
        assert_eq!(model.name, "Cube");
        assert_eq!(model.meshes[0].vertices.len(), 24);
        assert_eq!(model.meshes[0].indices.len(), 36);
    }

    #[test]
    fn test_plane() {
        let model = generate_plane(2.0, 2.0, 4);
        validate_model(&model);
        assert_eq!(model.name, "Plane");
        // (subdivisions+1)^2 = 5*5 = 25 vertices
        assert_eq!(model.meshes[0].vertices.len(), 25);
    }

    #[test]
    fn test_cylinder() {
        let model = generate_cylinder(0.5, 2.0, 16);
        validate_model(&model);
        assert_eq!(model.name, "Cylinder");
    }

    #[test]
    fn test_cone() {
        let model = generate_cone(0.5, 1.0, 16);
        validate_model(&model);
        assert_eq!(model.name, "Cone");
    }

    #[test]
    fn test_torus() {
        let model = generate_torus(1.0, 0.3, 24, 12);
        validate_model(&model);
        assert_eq!(model.name, "Torus");
    }

    #[test]
    fn test_generate_primitive_dispatch() {
        let shapes = ["sphere", "cube", "plane", "cylinder", "cone", "torus"];
        for shape in &shapes {
            let model = generate_primitive(shape, 1.0);
            validate_model(&model);
        }
        // Unknown shape defaults to cube.
        let model = generate_primitive("unknown_shape", 1.0);
        assert_eq!(model.name, "Cube");
    }
}
