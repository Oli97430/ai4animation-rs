//! BVH (BioVision Hierarchy) file parser.

use std::path::Path;
use glam::{Mat4, Vec3, Quat};
use anyhow::{Result, Context};
use crate::mesh::{ImportedModel, AnimationData};

pub struct BvhImporter;

struct BvhJoint {
    name: String,
    parent: i32,
    offset: Vec3,
    channels: Vec<ChannelType>,
    is_end_site: bool,
}

#[derive(Clone, Copy)]
enum ChannelType {
    Xposition, Yposition, Zposition,
    Xrotation, Yrotation, Zrotation,
}

impl BvhImporter {
    /// Load a BVH file with optional scale factor.
    pub fn load(path: &Path, scale: f32) -> Result<ImportedModel> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Impossible de lire: {}", path.display()))?;

        let file_name = path.file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "bvh".to_string());

        let mut joints: Vec<BvhJoint> = Vec::new();
        let mut channel_layout: Vec<(usize, ChannelType)> = Vec::new(); // (joint_idx, channel)
        let mut frames_data: Vec<Vec<f32>> = Vec::new();
        let mut framerate = 30.0f32;
        let mut _frame_count = 0usize;

        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;
        let mut parent_stack: Vec<i32> = vec![-1];
        let mut in_motion = false;

        while i < lines.len() {
            let line = lines[i].trim();

            if in_motion {
                if line.starts_with("Frames:") {
                    _frame_count = line.split(':').nth(1)
                        .and_then(|s| s.trim().parse().ok())
                        .unwrap_or(0);
                } else if line.starts_with("Frame Time:") {
                    let ft: f32 = line.split(':').nth(1)
                        .and_then(|s| s.trim().parse().ok())
                        .unwrap_or(1.0 / 30.0);
                    framerate = if ft <= 0.0 { 30.0 } else { 1.0 / ft };
                } else if !line.is_empty() {
                    let values: Vec<f32> = line.split_whitespace()
                        .filter_map(|s| s.parse().ok())
                        .collect();
                    if !values.is_empty() {
                        frames_data.push(values);
                    }
                }
            } else if line == "MOTION" {
                in_motion = true;
            } else if line.starts_with("ROOT") || line.starts_with("JOINT") {
                let name = line.split_whitespace().nth(1).unwrap_or("Joint").to_string();
                let parent = *parent_stack.last().unwrap_or(&-1);
                joints.push(BvhJoint {
                    name,
                    parent,
                    offset: Vec3::ZERO,
                    channels: Vec::new(),
                    is_end_site: false,
                });
            } else if line.starts_with("End Site") {
                let parent = *parent_stack.last().unwrap_or(&-1);
                joints.push(BvhJoint {
                    name: format!("{}_End", if parent >= 0 && (parent as usize) < joints.len() { &joints[parent as usize].name } else { "Root" }),
                    parent,
                    offset: Vec3::ZERO,
                    channels: Vec::new(),
                    is_end_site: true,
                });
            } else if line == "{" {
                let current = joints.len() as i32 - 1;
                parent_stack.push(current);
            } else if line == "}" {
                parent_stack.pop();
            } else if line.starts_with("OFFSET") {
                let parts: Vec<f32> = line.split_whitespace()
                    .skip(1)
                    .filter_map(|s| s.parse().ok())
                    .collect();
                if parts.len() >= 3 {
                    if let Some(joint) = joints.last_mut() {
                        joint.offset = Vec3::new(parts[0], parts[1], parts[2]) * scale;
                    }
                }
            } else if line.starts_with("CHANNELS") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                let joint_idx = joints.len() - 1;
                for ch_name in &parts[2..] {
                    let ch = match *ch_name {
                        "Xposition" => ChannelType::Xposition,
                        "Yposition" => ChannelType::Yposition,
                        "Zposition" => ChannelType::Zposition,
                        "Xrotation" => ChannelType::Xrotation,
                        "Yrotation" => ChannelType::Yrotation,
                        "Zrotation" => ChannelType::Zrotation,
                        _ => { i += 1; continue; }
                    };
                    joints[joint_idx].channels.push(ch);
                    channel_layout.push((joint_idx, ch));
                }
            }

            i += 1;
        }

        // Filter out end sites for the joint list
        let real_joints: Vec<usize> = (0..joints.len())
            .filter(|&i| !joints[i].is_end_site)
            .collect();

        let joint_names: Vec<String> = real_joints.iter().map(|&i| joints[i].name.clone()).collect();
        let old_to_new: std::collections::HashMap<usize, usize> = real_joints.iter()
            .enumerate()
            .map(|(new, &old)| (old, new))
            .collect();
        let parent_indices: Vec<i32> = real_joints.iter()
            .map(|&old| {
                let p = joints[old].parent;
                if p < 0 { -1 } else {
                    old_to_new.get(&(p as usize)).map(|&n| n as i32).unwrap_or(-1)
                }
            })
            .collect();

        // Build animation frames
        let _num_joints = real_joints.len();
        let num_all = joints.len();
        let mut frames = Vec::with_capacity(frames_data.len());

        for frame_values in &frames_data {
            // Per-joint translation and rotation
            let mut positions = vec![Vec3::ZERO; num_all];
            let mut euler_rotations = vec![Vec3::ZERO; num_all];

            // Apply offsets
            for j in 0..num_all {
                positions[j] = joints[j].offset;
            }

            // Apply channel data
            let mut ch_idx = 0;
            for &(joint_idx, ch_type) in &channel_layout {
                if ch_idx >= frame_values.len() { break; }
                let val = frame_values[ch_idx];
                match ch_type {
                    ChannelType::Xposition => positions[joint_idx].x = val * scale,
                    ChannelType::Yposition => positions[joint_idx].y = val * scale,
                    ChannelType::Zposition => positions[joint_idx].z = val * scale,
                    ChannelType::Xrotation => euler_rotations[joint_idx].x = val,
                    ChannelType::Yrotation => euler_rotations[joint_idx].y = val,
                    ChannelType::Zrotation => euler_rotations[joint_idx].z = val,
                }
                ch_idx += 1;
            }

            // Build local transforms and FK
            let mut local = vec![Mat4::IDENTITY; num_all];
            for j in 0..num_all {
                let e = euler_rotations[j];
                // BVH default: ZXY rotation order
                let rot = Quat::from_rotation_z(e.z.to_radians())
                    * Quat::from_rotation_x(e.x.to_radians())
                    * Quat::from_rotation_y(e.y.to_radians());
                local[j] = Mat4::from_rotation_translation(rot, positions[j]);
            }

            // FK to global
            let mut global = vec![Mat4::IDENTITY; num_all];
            for j in 0..num_all {
                let p = joints[j].parent;
                if p < 0 {
                    global[j] = local[j];
                } else {
                    global[j] = global[p as usize] * local[j];
                }
            }

            // Extract only real joints
            let frame: Vec<Mat4> = real_joints.iter().map(|&i| global[i]).collect();
            frames.push(frame);
        }

        let animation = if !frames.is_empty() {
            Some(AnimationData { frames, framerate })
        } else {
            None
        };

        Ok(ImportedModel {
            name: file_name,
            meshes: Vec::new(), // BVH has no mesh
            skin: None,
            joint_names,
            parent_indices,
            animation_frames: animation,
        })
    }
}
