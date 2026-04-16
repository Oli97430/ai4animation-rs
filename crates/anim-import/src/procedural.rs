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
