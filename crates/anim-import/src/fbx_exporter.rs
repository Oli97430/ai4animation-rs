//! ASCII FBX 7.4 exporter — write skeleton and animation to FBX format.
//!
//! Produces an ASCII FBX file from an `ImportedModel` + optional motion data (global-space
//! Mat4 frames). Joints are emitted as `LimbNode` Model objects; animation is written as
//! `AnimationStack` / `AnimationLayer` / `AnimationCurveNode` / `AnimationCurve` objects.

use std::io::Write;
use std::path::Path;

use glam::{Mat4, Vec3, EulerRot};

use crate::mesh::ImportedModel;

/// FBX time units: 1 second = 46_186_158_000 ticks.
const FBX_TIME_SECOND: i64 = 46_186_158_000;

// ── ID allocation ────────────────────────────────────────────

/// Simple monotonic ID allocator for FBX object IDs.
struct IdGen {
    next: i64,
}

impl IdGen {
    fn new() -> Self {
        Self { next: 100_000 }
    }

    fn next(&mut self) -> i64 {
        let id = self.next;
        self.next += 1;
        id
    }
}

// ── FBX writer helpers ──────────────────────────────────────

/// Write a line with the given indentation level.
fn wln(out: &mut Vec<u8>, indent: usize, line: &str) {
    for _ in 0..indent {
        out.extend_from_slice(b"\t");
    }
    out.extend_from_slice(line.as_bytes());
    out.extend_from_slice(b"\n");
}

/// Format a comma-separated list of f64 values (one per line, indented).
fn write_float_array(out: &mut Vec<u8>, indent: usize, values: &[f64]) {
    if values.is_empty() {
        return;
    }
    for _ in 0..indent {
        out.extend_from_slice(b"\t");
    }
    out.extend_from_slice(b"a: ");
    for (i, v) in values.iter().enumerate() {
        if i > 0 {
            out.extend_from_slice(b",");
        }
        let s = format!("{}", v);
        out.extend_from_slice(s.as_bytes());
    }
    out.extend_from_slice(b"\n");
}

/// Format a comma-separated list of i64 values.
fn write_i64_array(out: &mut Vec<u8>, indent: usize, values: &[i64]) {
    if values.is_empty() {
        return;
    }
    for _ in 0..indent {
        out.extend_from_slice(b"\t");
    }
    out.extend_from_slice(b"a: ");
    for (i, v) in values.iter().enumerate() {
        if i > 0 {
            out.extend_from_slice(b",");
        }
        let s = format!("{}", v);
        out.extend_from_slice(s.as_bytes());
    }
    out.extend_from_slice(b"\n");
}

// ── Global-to-local transform conversion ────────────────────

/// Decompose a local Mat4 into (translation, rotation_euler_xyz_degrees, scale).
fn decompose_trs(mat: &Mat4) -> (Vec3, Vec3, Vec3) {
    let (scale, rotation, translation) = mat.to_scale_rotation_translation();
    let (rx, ry, rz) = rotation.to_euler(EulerRot::XYZ);
    let rot_deg = Vec3::new(rx.to_degrees(), ry.to_degrees(), rz.to_degrees());
    (translation, rot_deg, scale)
}

/// Convert a frame of global transforms to local TRS per joint.
fn global_to_local(globals: &[Mat4], parent_indices: &[i32]) -> Vec<(Vec3, Vec3, Vec3)> {
    let num = globals.len().min(parent_indices.len());
    let mut locals = Vec::with_capacity(num);

    for i in 0..num {
        let local = if parent_indices[i] >= 0 {
            let pi = parent_indices[i] as usize;
            if pi < globals.len() {
                globals[pi].inverse() * globals[i]
            } else {
                globals[i]
            }
        } else {
            globals[i]
        };
        locals.push(decompose_trs(&local));
    }

    locals
}

// ── Public export function ──────────────────────────────────

/// Export a model (skeleton + optional animation) as an ASCII FBX 7.4 file.
///
/// - `model` — skeleton definition (joint names, parent indices)
/// - `frames` — optional global-space transforms `[frame_idx][joint_idx]`
/// - `framerate` — animation framerate (only used when `frames` is `Some`)
pub fn export_fbx(
    path: &Path,
    model: &ImportedModel,
    frames: Option<&Vec<Vec<Mat4>>>,
    framerate: f32,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut ids = IdGen::new();
    let mut out: Vec<u8> = Vec::new();

    let num_joints = model.joint_names.len();

    // Compute bind-pose local TRS (from first animation frame, or identity).
    let bind_locals: Vec<(Vec3, Vec3, Vec3)> = if let Some(fr) = frames {
        if !fr.is_empty() && !fr[0].is_empty() {
            global_to_local(&fr[0], &model.parent_indices)
        } else {
            vec![(Vec3::ZERO, Vec3::ZERO, Vec3::ONE); num_joints]
        }
    } else {
        vec![(Vec3::ZERO, Vec3::ZERO, Vec3::ONE); num_joints]
    };

    // ── Allocate IDs ────────────────────────────────────────
    let model_ids: Vec<i64> = (0..num_joints).map(|_| ids.next()).collect();

    // Animation IDs (only if we have frames)
    let has_anim = frames.map_or(false, |f| !f.is_empty() && num_joints > 0);
    let anim_stack_id = ids.next();
    let anim_layer_id = ids.next();

    // Per-joint: 3 curve node IDs (T, R, S) and 9 curve IDs (tx,ty,tz, rx,ry,rz, sx,sy,sz)
    struct JointAnimIds {
        cn_t: i64,
        cn_r: i64,
        cn_s: i64,
        curves: [i64; 9], // tx ty tz rx ry rz sx sy sz
    }

    let joint_anim_ids: Vec<JointAnimIds> = if has_anim {
        (0..num_joints)
            .map(|_| JointAnimIds {
                cn_t: ids.next(),
                cn_r: ids.next(),
                cn_s: ids.next(),
                curves: [
                    ids.next(),
                    ids.next(),
                    ids.next(),
                    ids.next(),
                    ids.next(),
                    ids.next(),
                    ids.next(),
                    ids.next(),
                    ids.next(),
                ],
            })
            .collect()
    } else {
        Vec::new()
    };

    // ── Header ──────────────────────────────────────────────
    wln(&mut out, 0, "; FBX 7.4.0 project file");
    wln(
        &mut out,
        0,
        "; Generated by AI4Animation Engine (Rust)",
    );
    wln(&mut out, 0, "");

    // FBXHeaderExtension
    wln(&mut out, 0, "FBXHeaderExtension:  {");
    wln(&mut out, 1, "FBXHeaderVersion: 1003");
    wln(&mut out, 1, "FBXVersion: 7400");
    wln(&mut out, 1, "Creator: \"AI4Animation Engine\"");
    wln(&mut out, 0, "}");
    wln(&mut out, 0, "");

    // GlobalSettings
    wln(&mut out, 0, "GlobalSettings:  {");
    wln(&mut out, 1, "Version: 1000");
    wln(&mut out, 1, "Properties70:  {");
    wln(
        &mut out,
        2,
        "P: \"UpAxis\", \"int\", \"Integer\", \"\",1",
    );
    wln(
        &mut out,
        2,
        "P: \"UpAxisSign\", \"int\", \"Integer\", \"\",1",
    );
    wln(
        &mut out,
        2,
        "P: \"FrontAxis\", \"int\", \"Integer\", \"\",2",
    );
    wln(
        &mut out,
        2,
        "P: \"FrontAxisSign\", \"int\", \"Integer\", \"\",1",
    );
    wln(
        &mut out,
        2,
        "P: \"CoordAxis\", \"int\", \"Integer\", \"\",0",
    );
    wln(
        &mut out,
        2,
        "P: \"CoordAxisSign\", \"int\", \"Integer\", \"\",1",
    );
    wln(
        &mut out,
        2,
        &format!(
            "P: \"CustomFrameRate\", \"double\", \"Number\", \"\",{}",
            framerate as f64
        ),
    );
    wln(&mut out, 1, "}");
    wln(&mut out, 0, "}");
    wln(&mut out, 0, "");

    // Documents
    wln(&mut out, 0, "Documents:  {");
    wln(&mut out, 1, "Count: 1");
    wln(&mut out, 1, "Document: 1000000, \"\", \"Scene\" {");
    wln(&mut out, 2, "Properties70:  {");
    wln(&mut out, 2, "}");
    wln(&mut out, 2, "RootNode: 0");
    wln(&mut out, 1, "}");
    wln(&mut out, 0, "}");
    wln(&mut out, 0, "");

    // ── Objects ─────────────────────────────────────────────
    wln(&mut out, 0, "Objects:  {");

    // Model nodes (LimbNode per joint)
    for (i, name) in model.joint_names.iter().enumerate() {
        let id = model_ids[i];
        let (t, r, s) = &bind_locals[i];
        wln(
            &mut out,
            1,
            &format!("Model: {}, \"Model::\\x00\\x01Model\", \"LimbNode\" {{", id),
        );
        wln(&mut out, 2, "Version: 232");
        wln(
            &mut out,
            2,
            &format!("Properties70:  {{"),
        );
        wln(
            &mut out,
            3,
            &format!(
                "P: \"Lcl Translation\", \"Lcl Translation\", \"\", \"A\",{},{},{}",
                t.x as f64, t.y as f64, t.z as f64
            ),
        );
        wln(
            &mut out,
            3,
            &format!(
                "P: \"Lcl Rotation\", \"Lcl Rotation\", \"\", \"A\",{},{},{}",
                r.x as f64, r.y as f64, r.z as f64
            ),
        );
        wln(
            &mut out,
            3,
            &format!(
                "P: \"Lcl Scaling\", \"Lcl Scaling\", \"\", \"A\",{},{},{}",
                s.x as f64, s.y as f64, s.z as f64
            ),
        );
        wln(&mut out, 2, "}");
        // Store the joint name as a comment for readability
        wln(
            &mut out,
            2,
            &format!("; Joint: {}", name),
        );
        wln(&mut out, 1, "}");
    }

    // Animation objects
    if has_anim {
        let anim_frames = frames.unwrap();
        let num_frames = anim_frames.len();
        let dt = 1.0 / framerate as f64;

        // Convert all frames to local TRS
        let all_local: Vec<Vec<(Vec3, Vec3, Vec3)>> = anim_frames
            .iter()
            .map(|frame| global_to_local(frame, &model.parent_indices))
            .collect();

        // AnimationStack
        wln(
            &mut out,
            1,
            &format!(
                "AnimationStack: {}, \"AnimStack::\\x00\\x01AnimStack\", \"\" {{",
                anim_stack_id
            ),
        );
        wln(&mut out, 2, "Properties70:  {");
        let total_time = (num_frames as f64 - 1.0) * dt;
        wln(
            &mut out,
            3,
            &format!(
                "P: \"LocalStop\", \"KTime\", \"Time\", \"\",{}",
                (total_time * FBX_TIME_SECOND as f64) as i64
            ),
        );
        wln(&mut out, 2, "}");
        wln(&mut out, 1, "}");

        // AnimationLayer
        wln(
            &mut out,
            1,
            &format!(
                "AnimationLayer: {}, \"AnimLayer::\\x00\\x01AnimLayer\", \"\" {{",
                anim_layer_id
            ),
        );
        wln(&mut out, 1, "}");

        // Precompute timestamps
        let timestamps: Vec<i64> = (0..num_frames)
            .map(|f| (f as f64 * dt * FBX_TIME_SECOND as f64) as i64)
            .collect();

        // Per-joint animation curve nodes and curves
        for j in 0..num_joints {
            let ja = &joint_anim_ids[j];

            // Collect per-component values
            let mut tx_vals = Vec::with_capacity(num_frames);
            let mut ty_vals = Vec::with_capacity(num_frames);
            let mut tz_vals = Vec::with_capacity(num_frames);
            let mut rx_vals = Vec::with_capacity(num_frames);
            let mut ry_vals = Vec::with_capacity(num_frames);
            let mut rz_vals = Vec::with_capacity(num_frames);
            let mut sx_vals = Vec::with_capacity(num_frames);
            let mut sy_vals = Vec::with_capacity(num_frames);
            let mut sz_vals = Vec::with_capacity(num_frames);

            for f in 0..num_frames {
                let (t, r, s) = &all_local[f][j];
                tx_vals.push(t.x as f64);
                ty_vals.push(t.y as f64);
                tz_vals.push(t.z as f64);
                rx_vals.push(r.x as f64);
                ry_vals.push(r.y as f64);
                rz_vals.push(r.z as f64);
                sx_vals.push(s.x as f64);
                sy_vals.push(s.y as f64);
                sz_vals.push(s.z as f64);
            }

            // AnimationCurveNode — T
            wln(
                &mut out,
                1,
                &format!(
                    "AnimationCurveNode: {}, \"AnimCurveNode::\\x00\\x01AnimCurveNode\", \"\" {{",
                    ja.cn_t
                ),
            );
            wln(&mut out, 2, "Properties70:  {");
            wln(
                &mut out,
                3,
                &format!(
                    "P: \"d|X\", \"Number\", \"\", \"A\",{}",
                    tx_vals[0]
                ),
            );
            wln(
                &mut out,
                3,
                &format!(
                    "P: \"d|Y\", \"Number\", \"\", \"A\",{}",
                    ty_vals[0]
                ),
            );
            wln(
                &mut out,
                3,
                &format!(
                    "P: \"d|Z\", \"Number\", \"\", \"A\",{}",
                    tz_vals[0]
                ),
            );
            wln(&mut out, 2, "}");
            wln(&mut out, 1, "}");

            // AnimationCurveNode — R
            wln(
                &mut out,
                1,
                &format!(
                    "AnimationCurveNode: {}, \"AnimCurveNode::\\x00\\x01AnimCurveNode\", \"\" {{",
                    ja.cn_r
                ),
            );
            wln(&mut out, 2, "Properties70:  {");
            wln(
                &mut out,
                3,
                &format!(
                    "P: \"d|X\", \"Number\", \"\", \"A\",{}",
                    rx_vals[0]
                ),
            );
            wln(
                &mut out,
                3,
                &format!(
                    "P: \"d|Y\", \"Number\", \"\", \"A\",{}",
                    ry_vals[0]
                ),
            );
            wln(
                &mut out,
                3,
                &format!(
                    "P: \"d|Z\", \"Number\", \"\", \"A\",{}",
                    rz_vals[0]
                ),
            );
            wln(&mut out, 2, "}");
            wln(&mut out, 1, "}");

            // AnimationCurveNode — S
            wln(
                &mut out,
                1,
                &format!(
                    "AnimationCurveNode: {}, \"AnimCurveNode::\\x00\\x01AnimCurveNode\", \"\" {{",
                    ja.cn_s
                ),
            );
            wln(&mut out, 2, "Properties70:  {");
            wln(
                &mut out,
                3,
                &format!(
                    "P: \"d|X\", \"Number\", \"\", \"A\",{}",
                    sx_vals[0]
                ),
            );
            wln(
                &mut out,
                3,
                &format!(
                    "P: \"d|Y\", \"Number\", \"\", \"A\",{}",
                    sy_vals[0]
                ),
            );
            wln(
                &mut out,
                3,
                &format!(
                    "P: \"d|Z\", \"Number\", \"\", \"A\",{}",
                    sz_vals[0]
                ),
            );
            wln(&mut out, 2, "}");
            wln(&mut out, 1, "}");

            // 9 AnimationCurves: tx ty tz rx ry rz sx sy sz
            let curve_values: [&Vec<f64>; 9] = [
                &tx_vals, &ty_vals, &tz_vals, &rx_vals, &ry_vals, &rz_vals, &sx_vals, &sy_vals,
                &sz_vals,
            ];

            for (ci, vals) in curve_values.iter().enumerate() {
                let curve_id = ja.curves[ci];
                wln(
                    &mut out,
                    1,
                    &format!(
                        "AnimationCurve: {}, \"AnimCurve::\\x00\\x01AnimCurve\", \"\" {{",
                        curve_id
                    ),
                );
                wln(&mut out, 2, &format!("KeyVer: 4009"));
                wln(
                    &mut out,
                    2,
                    &format!("KeyTime: *{} {{", num_frames),
                );
                write_i64_array(&mut out, 3, &timestamps);
                wln(&mut out, 2, "}");
                wln(
                    &mut out,
                    2,
                    &format!("KeyValueFloat: *{} {{", num_frames),
                );
                write_float_array(&mut out, 3, vals);
                wln(&mut out, 2, "}");
                wln(&mut out, 1, "}");
            }
        }
    }

    wln(&mut out, 0, "}");
    wln(&mut out, 0, "");

    // ── Connections ─────────────────────────────────────────
    wln(&mut out, 0, "Connections:  {");

    // Connect model nodes to scene root (0) or parent model
    for (i, &parent) in model.parent_indices.iter().enumerate() {
        let child_id = model_ids[i];
        let parent_id = if parent < 0 {
            0 // scene root
        } else {
            model_ids[parent as usize]
        };
        wln(
            &mut out,
            1,
            &format!("C: \"OO\",{},{}", child_id, parent_id),
        );
    }

    // Animation connections
    if has_anim {
        // AnimStack -> scene root
        wln(
            &mut out,
            1,
            &format!("C: \"OO\",{},0", anim_stack_id),
        );
        // AnimLayer -> AnimStack
        wln(
            &mut out,
            1,
            &format!("C: \"OO\",{},{}", anim_layer_id, anim_stack_id),
        );

        for j in 0..num_joints {
            let ja = &joint_anim_ids[j];
            let model_id = model_ids[j];

            // CurveNode T -> Model (property connection)
            wln(
                &mut out,
                1,
                &format!(
                    "C: \"OP\",{},{}, \"Lcl Translation\"",
                    ja.cn_t, model_id
                ),
            );
            // CurveNode R -> Model
            wln(
                &mut out,
                1,
                &format!(
                    "C: \"OP\",{},{}, \"Lcl Rotation\"",
                    ja.cn_r, model_id
                ),
            );
            // CurveNode S -> Model
            wln(
                &mut out,
                1,
                &format!(
                    "C: \"OP\",{},{}, \"Lcl Scaling\"",
                    ja.cn_s, model_id
                ),
            );

            // CurveNode T -> AnimLayer
            wln(
                &mut out,
                1,
                &format!("C: \"OO\",{},{}", ja.cn_t, anim_layer_id),
            );
            // CurveNode R -> AnimLayer
            wln(
                &mut out,
                1,
                &format!("C: \"OO\",{},{}", ja.cn_r, anim_layer_id),
            );
            // CurveNode S -> AnimLayer
            wln(
                &mut out,
                1,
                &format!("C: \"OO\",{},{}", ja.cn_s, anim_layer_id),
            );

            // Curves -> CurveNodes (with d|X/d|Y/d|Z property connections)
            let component_labels = ["d|X", "d|Y", "d|Z"];
            // T curves (0..3) -> cn_t
            for (ci, label) in component_labels.iter().enumerate() {
                wln(
                    &mut out,
                    1,
                    &format!(
                        "C: \"OP\",{},{}, \"{}\"",
                        ja.curves[ci], ja.cn_t, label
                    ),
                );
            }
            // R curves (3..6) -> cn_r
            for (ci, label) in component_labels.iter().enumerate() {
                wln(
                    &mut out,
                    1,
                    &format!(
                        "C: \"OP\",{},{}, \"{}\"",
                        ja.curves[3 + ci], ja.cn_r, label
                    ),
                );
            }
            // S curves (6..9) -> cn_s
            for (ci, label) in component_labels.iter().enumerate() {
                wln(
                    &mut out,
                    1,
                    &format!(
                        "C: \"OP\",{},{}, \"{}\"",
                        ja.curves[6 + ci], ja.cn_s, label
                    ),
                );
            }
        }
    }

    wln(&mut out, 0, "}");
    wln(&mut out, 0, "");

    // ── Write file ──────────────────────────────────────────
    let mut file = std::fs::File::create(path)?;
    file.write_all(&out)?;
    file.flush()?;

    Ok(())
}

// ── Tests ───────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::ImportedModel;
    use glam::Mat4;

    fn empty_model(joint_names: Vec<String>, parent_indices: Vec<i32>) -> ImportedModel {
        ImportedModel {
            name: "test".into(),
            meshes: Vec::new(),
            skin: None,
            joint_names,
            parent_indices,
            animation_frames: None,
        }
    }

    #[test]
    fn test_export_empty() {
        // Model with no joints should not crash.
        let model = empty_model(Vec::new(), Vec::new());
        let dir = std::env::temp_dir();
        let path = dir.join("test_fbx_empty.fbx");
        let result = export_fbx(&path, &model, None, 30.0);
        assert!(result.is_ok(), "export_fbx failed: {:?}", result.err());
        // File should exist and contain FBX header
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("FBX 7.4.0"));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_export_skeleton_only() {
        // Export with joints but no animation, verify file is written.
        let model = empty_model(
            vec!["Hips".into(), "Spine".into(), "Head".into()],
            vec![-1, 0, 1],
        );
        let dir = std::env::temp_dir();
        let path = dir.join("test_fbx_skeleton.fbx");
        let result = export_fbx(&path, &model, None, 30.0);
        assert!(result.is_ok(), "export_fbx failed: {:?}", result.err());

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("FBXHeaderExtension"));
        assert!(content.contains("Objects:"));
        assert!(content.contains("Connections:"));
        // Should have 3 Model nodes
        // Check for LimbNode entries (more reliable than matching "Model:")
        let limb_count = content.matches("LimbNode").count();
        assert_eq!(limb_count, 3, "Expected 3 LimbNode entries");

        // No animation objects expected
        assert!(!content.contains("AnimationStack"));

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_export_with_animation() {
        // Export with 2 joints and 3 frames of animation.
        let model = empty_model(
            vec!["Root".into(), "Child".into()],
            vec![-1, 0],
        );

        // 3 frames, 2 joints each (global space)
        let frames: Vec<Vec<Mat4>> = vec![
            vec![
                Mat4::from_translation(glam::Vec3::new(0.0, 0.0, 0.0)),
                Mat4::from_translation(glam::Vec3::new(0.0, 1.0, 0.0)),
            ],
            vec![
                Mat4::from_translation(glam::Vec3::new(0.0, 0.0, 0.0)),
                Mat4::from_translation(glam::Vec3::new(0.0, 2.0, 0.0)),
            ],
            vec![
                Mat4::from_translation(glam::Vec3::new(0.0, 0.0, 0.0)),
                Mat4::from_translation(glam::Vec3::new(0.0, 3.0, 0.0)),
            ],
        ];

        let dir = std::env::temp_dir();
        let path = dir.join("test_fbx_animation.fbx");
        let result = export_fbx(&path, &model, Some(&frames), 30.0);
        assert!(result.is_ok(), "export_fbx failed: {:?}", result.err());

        let content = std::fs::read_to_string(&path).unwrap();
        // Should contain animation objects
        assert!(content.contains("AnimationStack"), "Missing AnimationStack");
        assert!(content.contains("AnimationLayer"), "Missing AnimationLayer");
        assert!(content.contains("AnimationCurveNode"), "Missing AnimationCurveNode");
        assert!(content.contains("AnimationCurve"), "Missing AnimationCurve");
        assert!(content.contains("KeyTime"), "Missing KeyTime");
        assert!(content.contains("KeyValueFloat"), "Missing KeyValueFloat");

        // Should have 2 LimbNode entries (one per joint)
        let limb_count = content.matches("LimbNode").count();
        assert_eq!(limb_count, 2, "Expected 2 LimbNode entries");

        // Should have 6 AnimCurveNode entries (T, R, S per 2 joints)
        let cn_count = content.matches("AnimCurveNode:").count();
        // In connections section too, so just check Objects section has at least 6
        assert!(cn_count >= 6, "Expected at least 6 AnimCurveNode references, got {}", cn_count);

        std::fs::remove_file(&path).ok();
    }
}
