//! BVH file exporter — write a skeleton pose/animation to BVH format.

use anyhow::Result;
use glam::{Mat4, Quat, EulerRot};
use std::io::Write;
use std::path::Path;

use anim_math::transform::Transform;

/// Export a single-frame BVH from joint names, parent indices, and transforms.
pub fn export_bvh_pose(
    path: &Path,
    joint_names: &[String],
    parent_indices: &[i32],
    transforms: &[Mat4],
    framerate: f32,
) -> Result<()> {
    let mut file = std::fs::File::create(path)?;

    // Write HIERARCHY section
    writeln!(file, "HIERARCHY")?;

    // Build children map
    let num_joints = joint_names.len();
    let mut children: Vec<Vec<usize>> = vec![Vec::new(); num_joints];
    let mut roots = Vec::new();
    for (i, &parent) in parent_indices.iter().enumerate() {
        if parent < 0 {
            roots.push(i);
        } else {
            children[parent as usize].push(i);
        }
    }

    // Write hierarchy recursively
    for &root in &roots {
        write_joint(&mut file, root, joint_names, &children, transforms, parent_indices, 0, true)?;
    }

    // Write MOTION section (single frame)
    writeln!(file, "MOTION")?;
    writeln!(file, "Frames: 1")?;
    writeln!(file, "Frame Time: {:.6}", 1.0 / framerate)?;

    // Write frame data: for each joint, output position (for root) + rotations
    let mut values = Vec::new();
    for i in 0..num_joints {
        let local = if parent_indices[i] < 0 {
            transforms[i]
        } else {
            let parent_inv = transforms[parent_indices[i] as usize].inverse();
            parent_inv * transforms[i]
        };

        let pos = local.get_position();
        let quat = Quat::from_mat4(&local);
        let (rz, rx, ry) = quat.to_euler(EulerRot::ZXY);

        // Root: position + rotation
        if parent_indices[i] < 0 {
            values.push(format!("{:.6}", pos.x));
            values.push(format!("{:.6}", pos.y));
            values.push(format!("{:.6}", pos.z));
        }
        values.push(format!("{:.6}", rz.to_degrees()));
        values.push(format!("{:.6}", rx.to_degrees()));
        values.push(format!("{:.6}", ry.to_degrees()));
    }

    writeln!(file, "{}", values.join(" "))?;

    Ok(())
}

/// Export a full BVH animation sequence (all frames).
pub fn export_bvh_sequence(
    path: &Path,
    joint_names: &[String],
    parent_indices: &[i32],
    frames: &[Vec<Mat4>],
    framerate: f32,
) -> Result<()> {
    if frames.is_empty() {
        anyhow::bail!("No frames to export");
    }

    let mut file = std::fs::File::create(path)?;
    let num_joints = joint_names.len();

    // Write HIERARCHY section (using first frame for offsets)
    writeln!(file, "HIERARCHY")?;

    let mut children: Vec<Vec<usize>> = vec![Vec::new(); num_joints];
    let mut roots = Vec::new();
    for (i, &parent) in parent_indices.iter().enumerate() {
        if parent < 0 {
            roots.push(i);
        } else {
            children[parent as usize].push(i);
        }
    }

    let ref_frame = &frames[0];
    for &root in &roots {
        write_joint(&mut file, root, joint_names, &children, ref_frame, parent_indices, 0, true)?;
    }

    // Write MOTION section
    writeln!(file, "MOTION")?;
    writeln!(file, "Frames: {}", frames.len())?;
    writeln!(file, "Frame Time: {:.6}", 1.0 / framerate)?;

    // Write all frames
    for frame in frames {
        let mut values = Vec::new();
        for i in 0..num_joints {
            let local = if parent_indices[i] < 0 {
                frame[i]
            } else {
                let parent_inv = frame[parent_indices[i] as usize].inverse();
                parent_inv * frame[i]
            };

            let pos = local.get_position();
            let quat = Quat::from_mat4(&local);
            let (rz, rx, ry) = quat.to_euler(EulerRot::ZXY);

            if parent_indices[i] < 0 {
                values.push(format!("{:.6}", pos.x));
                values.push(format!("{:.6}", pos.y));
                values.push(format!("{:.6}", pos.z));
            }
            values.push(format!("{:.6}", rz.to_degrees()));
            values.push(format!("{:.6}", rx.to_degrees()));
            values.push(format!("{:.6}", ry.to_degrees()));
        }

        writeln!(file, "{}", values.join(" "))?;
    }

    log::info!("BVH export: {} ({} frames, {} joints)", path.display(), frames.len(), num_joints);
    Ok(())
}

fn write_joint(
    file: &mut std::fs::File,
    joint_idx: usize,
    names: &[String],
    children: &[Vec<usize>],
    transforms: &[Mat4],
    parent_indices: &[i32],
    depth: usize,
    is_root: bool,
) -> Result<()> {
    let indent = "  ".repeat(depth);
    let name = &names[joint_idx];

    // Compute offset from parent
    let offset = if parent_indices[joint_idx] < 0 {
        transforms[joint_idx].get_position()
    } else {
        let parent_pos = transforms[parent_indices[joint_idx] as usize].get_position();
        transforms[joint_idx].get_position() - parent_pos
    };

    if is_root {
        writeln!(file, "{}ROOT {}", indent, name)?;
    } else {
        writeln!(file, "{}JOINT {}", indent, name)?;
    }

    writeln!(file, "{}{{", indent)?;
    writeln!(file, "{}  OFFSET {:.6} {:.6} {:.6}", indent, offset.x, offset.y, offset.z)?;

    if is_root {
        writeln!(file, "{}  CHANNELS 6 Xposition Yposition Zposition Zrotation Xrotation Yrotation", indent)?;
    } else {
        writeln!(file, "{}  CHANNELS 3 Zrotation Xrotation Yrotation", indent)?;
    }

    if children[joint_idx].is_empty() {
        // End site
        writeln!(file, "{}  End Site", indent)?;
        writeln!(file, "{}  {{", indent)?;
        writeln!(file, "{}    OFFSET 0.000000 0.100000 0.000000", indent)?;
        writeln!(file, "{}  }}", indent)?;
    } else {
        for &child in &children[joint_idx] {
            write_joint(file, child, names, children, transforms, parent_indices, depth + 1, false)?;
        }
    }

    writeln!(file, "{}}}", indent)?;
    Ok(())
}
