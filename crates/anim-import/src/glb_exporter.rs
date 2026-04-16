//! GLB (Binary glTF 2.0) exporter — write mesh, skeleton, skinning, and animation.
//!
//! Produces a single self-contained .glb file from an ImportedModel + optional Motion data.

use anyhow::Result;
use glam::{Mat4, Vec3, Quat};
use serde_json::{json, Value as Json};
use std::io::Write;
use std::path::Path;

use crate::mesh::{ImportedModel, ImportedMesh};

/// Export a complete model as GLB.
///
/// - `model` — meshes, skin, skeleton
/// - `frames` — optional animation frames [num_frames][num_joints] in global space
/// - `framerate` — animation framerate (used only when frames is Some)
pub fn export_glb(
    path: &Path,
    model: &ImportedModel,
    frames: Option<&Vec<Vec<Mat4>>>,
    framerate: f32,
) -> Result<()> {
    let mut bin = BinWriter::new();
    let mut json = GltfBuilder::new();

    let num_joints = model.joint_names.len();

    // ── Scene hierarchy ──────────────────────────────────
    // One scene, one root node. Skeleton joints are separate nodes.
    let mut all_nodes: Vec<Json> = Vec::new();
    let mut joint_node_indices: Vec<usize> = Vec::new();

    // Build joint nodes
    for (i, name) in model.joint_names.iter().enumerate() {
        let node_idx = all_nodes.len();
        joint_node_indices.push(node_idx);
        let mut node = json!({ "name": name });

        // Add children
        let children: Vec<usize> = model.parent_indices.iter().enumerate()
            .filter(|(_, &p)| p == i as i32)
            .map(|(ci, _)| ci) // child joint index — will be remapped below
            .collect();
        // Children must reference node indices, not joint indices.
        // Since joint nodes are placed sequentially starting at 0, joint_index == node_index
        // for the first N nodes.
        if !children.is_empty() {
            node["children"] = json!(children);
        }

        all_nodes.push(node);
    }

    // Build mesh nodes (one per mesh)
    let mut mesh_node_indices: Vec<usize> = Vec::new();
    for (mi, _mesh) in model.meshes.iter().enumerate() {
        let node_idx = all_nodes.len();
        mesh_node_indices.push(node_idx);
        let mut node = json!({ "name": format!("Mesh_{}", mi), "mesh": mi });
        if model.skin.is_some() {
            node["skin"] = json!(0);
        }
        all_nodes.push(node);
    }

    // Scene root: children are root joints + mesh nodes
    let root_joints: Vec<usize> = model.parent_indices.iter().enumerate()
        .filter(|(_, &p)| p < 0)
        .map(|(i, _)| joint_node_indices[i])
        .collect();
    let mut scene_children = root_joints.clone();
    scene_children.extend_from_slice(&mesh_node_indices);

    // Don't add a scene root node — just put children in scene directly
    json.scenes.push(json!({ "nodes": scene_children }));

    // ── Meshes ───────────────────────────────────────────
    for mesh in &model.meshes {
        let (mesh_json, _) = write_mesh(&mut bin, &mut json, mesh, model.skin.is_some())?;
        json.meshes.push(mesh_json);
    }

    // ── Skin ─────────────────────────────────────────────
    if let Some(ref skin) = model.skin {
        let ibm_accessor = write_inverse_bind_matrices(&mut bin, &mut json, &skin.inverse_bind_matrices)?;
        let joints: Vec<usize> = (0..num_joints).collect();
        let skeleton_root = root_joints.first().copied();
        let mut skin_json = json!({
            "inverseBindMatrices": ibm_accessor,
            "joints": joints,
        });
        if let Some(root) = skeleton_root {
            skin_json["skeleton"] = json!(root);
        }
        json.skins.push(skin_json);
    }

    // ── Animation ────────────────────────────────────────
    if let Some(anim_frames) = frames {
        if !anim_frames.is_empty() && num_joints > 0 {
            write_animation(&mut bin, &mut json, anim_frames, framerate, num_joints, &model.parent_indices)?;
        }
    }

    // ── Assemble nodes into JSON ─────────────────────────
    json.nodes = all_nodes;

    // ── Write GLB ────────────────────────────────────────
    let bin_data = bin.finish();
    let json_str = json.build(&bin_data)?;

    write_glb_file(path, &json_str, &bin_data)?;

    Ok(())
}

/// Export only the skeleton + animation (no mesh) as GLB.
pub fn export_glb_skeleton(
    path: &Path,
    joint_names: &[String],
    parent_indices: &[i32],
    frames: &[Vec<Mat4>],
    framerate: f32,
) -> Result<()> {
    let model = ImportedModel {
        name: "skeleton".into(),
        meshes: Vec::new(),
        skin: None,
        joint_names: joint_names.to_vec(),
        parent_indices: parent_indices.to_vec(),
        animation_frames: None,
    };
    let frames_vec = frames.to_vec();
    let frames_ref = if frames.is_empty() { None } else { Some(&frames_vec) };
    export_glb(path, &model, frames_ref, framerate)
}

// ── Binary buffer writer ─────────────────────────────────

struct BinWriter {
    data: Vec<u8>,
}

impl BinWriter {
    fn new() -> Self {
        Self { data: Vec::new() }
    }

    /// Align to 4-byte boundary.
    fn align4(&mut self) {
        while self.data.len() % 4 != 0 {
            self.data.push(0);
        }
    }

    /// Current byte offset.
    fn offset(&self) -> usize {
        self.data.len()
    }

    fn write_f32_slice(&mut self, values: &[f32]) {
        for &v in values {
            self.data.extend_from_slice(&v.to_le_bytes());
        }
    }

    fn write_u16_slice(&mut self, values: &[u16]) {
        for &v in values {
            self.data.extend_from_slice(&v.to_le_bytes());
        }
    }

    fn write_u32_slice(&mut self, values: &[u32]) {
        for &v in values {
            self.data.extend_from_slice(&v.to_le_bytes());
        }
    }

    fn write_u8_slice(&mut self, values: &[u8]) {
        self.data.extend_from_slice(values);
    }

    fn finish(mut self) -> Vec<u8> {
        self.align4();
        self.data
    }
}

// ── glTF JSON builder ────────────────────────────────────

struct GltfBuilder {
    scenes: Vec<Json>,
    nodes: Vec<Json>,
    meshes: Vec<Json>,
    skins: Vec<Json>,
    animations: Vec<Json>,
    accessors: Vec<Json>,
    buffer_views: Vec<Json>,
}

impl GltfBuilder {
    fn new() -> Self {
        Self {
            scenes: Vec::new(),
            nodes: Vec::new(),
            meshes: Vec::new(),
            skins: Vec::new(),
            animations: Vec::new(),
            accessors: Vec::new(),
            buffer_views: Vec::new(),
        }
    }

    fn add_buffer_view(&mut self, byte_offset: usize, byte_length: usize, target: Option<u32>) -> usize {
        let idx = self.buffer_views.len();
        let mut bv = json!({
            "buffer": 0,
            "byteOffset": byte_offset,
            "byteLength": byte_length,
        });
        if let Some(t) = target {
            bv["target"] = json!(t);
        }
        self.buffer_views.push(bv);
        idx
    }

    fn add_accessor(
        &mut self,
        buffer_view: usize,
        byte_offset: usize,
        component_type: u32,
        count: usize,
        accessor_type: &str,
        min: Option<Vec<f32>>,
        max: Option<Vec<f32>>,
    ) -> usize {
        let idx = self.accessors.len();
        let mut acc = json!({
            "bufferView": buffer_view,
            "byteOffset": byte_offset,
            "componentType": component_type,
            "count": count,
            "type": accessor_type,
        });
        if let Some(min_val) = min {
            acc["min"] = json!(min_val);
        }
        if let Some(max_val) = max {
            acc["max"] = json!(max_val);
        }
        self.accessors.push(acc);
        idx
    }

    fn build(&self, bin_data: &[u8]) -> Result<String> {
        let mut root = json!({
            "asset": {
                "version": "2.0",
                "generator": "AI4Animation Engine (Rust)"
            },
            "scene": 0,
            "scenes": self.scenes,
            "buffers": [{
                "byteLength": bin_data.len()
            }]
        });

        if !self.nodes.is_empty() {
            root["nodes"] = json!(self.nodes);
        }
        if !self.meshes.is_empty() {
            root["meshes"] = json!(self.meshes);
        }
        if !self.skins.is_empty() {
            root["skins"] = json!(self.skins);
        }
        if !self.animations.is_empty() {
            root["animations"] = json!(self.animations);
        }
        if !self.accessors.is_empty() {
            root["accessors"] = json!(self.accessors);
        }
        if !self.buffer_views.is_empty() {
            root["bufferViews"] = json!(self.buffer_views);
        }

        Ok(serde_json::to_string(&root)?)
    }
}

// ── Write mesh data ──────────────────────────────────────

/// glTF component types
const FLOAT: u32 = 5126;
const UNSIGNED_SHORT: u32 = 5123;
const UNSIGNED_INT: u32 = 5125;
const UNSIGNED_BYTE: u32 = 5121;

/// glTF buffer view targets
const ARRAY_BUFFER: u32 = 34962;
const ELEMENT_ARRAY_BUFFER: u32 = 34963;

fn write_mesh(
    bin: &mut BinWriter,
    json: &mut GltfBuilder,
    mesh: &ImportedMesh,
    has_skin: bool,
) -> Result<(Json, usize)> {
    let num_vertices = mesh.vertices.len();

    // Positions
    bin.align4();
    let pos_offset = bin.offset();
    let mut pos_min = Vec3::splat(f32::MAX);
    let mut pos_max = Vec3::splat(f32::MIN);
    for &v in &mesh.vertices {
        pos_min = pos_min.min(v);
        pos_max = pos_max.max(v);
        bin.write_f32_slice(&[v.x, v.y, v.z]);
    }
    let pos_len = bin.offset() - pos_offset;
    let pos_bv = json.add_buffer_view(pos_offset, pos_len, Some(ARRAY_BUFFER));
    let pos_acc = json.add_accessor(
        pos_bv, 0, FLOAT, num_vertices, "VEC3",
        Some(vec![pos_min.x, pos_min.y, pos_min.z]),
        Some(vec![pos_max.x, pos_max.y, pos_max.z]),
    );

    // Normals
    bin.align4();
    let norm_offset = bin.offset();
    for n in &mesh.normals {
        bin.write_f32_slice(&[n.x, n.y, n.z]);
    }
    let norm_len = bin.offset() - norm_offset;
    let norm_bv = json.add_buffer_view(norm_offset, norm_len, Some(ARRAY_BUFFER));
    let norm_acc = json.add_accessor(norm_bv, 0, FLOAT, mesh.normals.len(), "VEC3", None, None);

    // Texcoords
    let mut attributes = json!({
        "POSITION": pos_acc,
        "NORMAL": norm_acc,
    });

    if !mesh.texcoords.is_empty() {
        bin.align4();
        let tc_offset = bin.offset();
        for tc in &mesh.texcoords {
            bin.write_f32_slice(&[tc.x, tc.y]);
        }
        let tc_len = bin.offset() - tc_offset;
        let tc_bv = json.add_buffer_view(tc_offset, tc_len, Some(ARRAY_BUFFER));
        let tc_acc = json.add_accessor(tc_bv, 0, FLOAT, mesh.texcoords.len(), "VEC2", None, None);
        attributes["TEXCOORD_0"] = json!(tc_acc);
    }

    // Bone weights & indices (for skinning)
    if has_skin && !mesh.bone_weights.is_empty() {
        // JOINTS_0 as unsigned byte (max 255 joints)
        bin.align4();
        let joints_offset = bin.offset();
        for bi in &mesh.bone_indices {
            bin.write_u8_slice(&[bi[0] as u8, bi[1] as u8, bi[2] as u8, bi[3] as u8]);
        }
        let joints_len = bin.offset() - joints_offset;
        let joints_bv = json.add_buffer_view(joints_offset, joints_len, Some(ARRAY_BUFFER));
        let joints_acc = json.add_accessor(joints_bv, 0, UNSIGNED_BYTE, num_vertices, "VEC4", None, None);
        attributes["JOINTS_0"] = json!(joints_acc);

        // WEIGHTS_0 as float
        bin.align4();
        let weights_offset = bin.offset();
        for bw in &mesh.bone_weights {
            bin.write_f32_slice(&[bw[0], bw[1], bw[2], bw[3]]);
        }
        let weights_len = bin.offset() - weights_offset;
        let weights_bv = json.add_buffer_view(weights_offset, weights_len, Some(ARRAY_BUFFER));
        let weights_acc = json.add_accessor(weights_bv, 0, FLOAT, num_vertices, "VEC4", None, None);
        attributes["WEIGHTS_0"] = json!(weights_acc);
    }

    // Indices
    bin.align4();
    let idx_offset = bin.offset();
    let num_indices = mesh.indices.len();
    // Use u16 if possible (< 65536 vertices), else u32
    let (idx_component_type, idx_byte_len) = if num_vertices <= 65535 {
        let u16_indices: Vec<u16> = mesh.indices.iter().map(|&i| i as u16).collect();
        bin.write_u16_slice(&u16_indices);
        (UNSIGNED_SHORT, num_indices * 2)
    } else {
        bin.write_u32_slice(&mesh.indices);
        (UNSIGNED_INT, num_indices * 4)
    };
    let _ = idx_byte_len; // used implicitly through bin.offset()
    let idx_len = bin.offset() - idx_offset;
    let idx_bv = json.add_buffer_view(idx_offset, idx_len, Some(ELEMENT_ARRAY_BUFFER));
    let idx_acc = json.add_accessor(idx_bv, 0, idx_component_type, num_indices, "SCALAR", None, None);

    let mesh_json = json!({
        "primitives": [{
            "attributes": attributes,
            "indices": idx_acc,
            "mode": 4  // TRIANGLES
        }]
    });

    Ok((mesh_json, 0))
}

// ── Write inverse bind matrices ──────────────────────────

fn write_inverse_bind_matrices(
    bin: &mut BinWriter,
    json: &mut GltfBuilder,
    ibms: &[Mat4],
) -> Result<usize> {
    bin.align4();
    let offset = bin.offset();
    for m in ibms {
        let cols = m.to_cols_array();
        bin.write_f32_slice(&cols);
    }
    let len = bin.offset() - offset;
    let bv = json.add_buffer_view(offset, len, None);
    let acc = json.add_accessor(bv, 0, FLOAT, ibms.len(), "MAT4", None, None);
    Ok(acc)
}

// ── Write animation ──────────────────────────────────────

fn write_animation(
    bin: &mut BinWriter,
    json: &mut GltfBuilder,
    frames: &[Vec<Mat4>],
    framerate: f32,
    num_joints: usize,
    parent_indices: &[i32],
) -> Result<()> {
    let num_frames = frames.len();
    if num_frames == 0 || num_joints == 0 {
        return Ok(());
    }

    let dt = 1.0 / framerate;

    // First, convert global transforms to local transforms
    let local_frames: Vec<Vec<(Vec3, Quat, Vec3)>> = frames.iter().map(|frame| {
        global_to_local(frame, parent_indices)
    }).collect();

    // Time stamps (shared by all channels)
    bin.align4();
    let time_offset = bin.offset();
    let times: Vec<f32> = (0..num_frames).map(|i| i as f32 * dt).collect();
    bin.write_f32_slice(&times);
    let time_len = bin.offset() - time_offset;
    let time_bv = json.add_buffer_view(time_offset, time_len, None);
    let time_acc = json.add_accessor(
        time_bv, 0, FLOAT, num_frames, "SCALAR",
        Some(vec![0.0]),
        Some(vec![times.last().copied().unwrap_or(0.0)]),
    );

    let mut samplers: Vec<Json> = Vec::new();
    let mut channels: Vec<Json> = Vec::new();

    for joint in 0..num_joints {
        // Translation
        bin.align4();
        let t_offset = bin.offset();
        for f in 0..num_frames {
            let (t, _, _) = &local_frames[f][joint];
            bin.write_f32_slice(&[t.x, t.y, t.z]);
        }
        let t_len = bin.offset() - t_offset;
        let t_bv = json.add_buffer_view(t_offset, t_len, None);
        let t_acc = json.add_accessor(t_bv, 0, FLOAT, num_frames, "VEC3", None, None);
        let t_sampler = samplers.len();
        samplers.push(json!({ "input": time_acc, "output": t_acc, "interpolation": "LINEAR" }));
        channels.push(json!({
            "sampler": t_sampler,
            "target": { "node": joint, "path": "translation" }
        }));

        // Rotation (quaternion)
        bin.align4();
        let r_offset = bin.offset();
        for f in 0..num_frames {
            let (_, r, _) = &local_frames[f][joint];
            bin.write_f32_slice(&[r.x, r.y, r.z, r.w]);
        }
        let r_len = bin.offset() - r_offset;
        let r_bv = json.add_buffer_view(r_offset, r_len, None);
        let r_acc = json.add_accessor(r_bv, 0, FLOAT, num_frames, "VEC4", None, None);
        let r_sampler = samplers.len();
        samplers.push(json!({ "input": time_acc, "output": r_acc, "interpolation": "LINEAR" }));
        channels.push(json!({
            "sampler": r_sampler,
            "target": { "node": joint, "path": "rotation" }
        }));

        // Scale (only if non-uniform)
        bin.align4();
        let s_offset = bin.offset();
        for f in 0..num_frames {
            let (_, _, s) = &local_frames[f][joint];
            bin.write_f32_slice(&[s.x, s.y, s.z]);
        }
        let s_len = bin.offset() - s_offset;
        let s_bv = json.add_buffer_view(s_offset, s_len, None);
        let s_acc = json.add_accessor(s_bv, 0, FLOAT, num_frames, "VEC3", None, None);
        let s_sampler = samplers.len();
        samplers.push(json!({ "input": time_acc, "output": s_acc, "interpolation": "LINEAR" }));
        channels.push(json!({
            "sampler": s_sampler,
            "target": { "node": joint, "path": "scale" }
        }));
    }

    json.animations.push(json!({
        "name": "Animation",
        "samplers": samplers,
        "channels": channels,
    }));

    Ok(())
}

/// Convert global transforms to local (parent-relative) TRS decomposition.
fn global_to_local(globals: &[Mat4], parent_indices: &[i32]) -> Vec<(Vec3, Quat, Vec3)> {
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

        let (scale, rotation, translation) = local.to_scale_rotation_translation();
        locals.push((translation, rotation, scale));
    }

    locals
}

// ── Write GLB file ───────────────────────────────────────

fn write_glb_file(path: &Path, json_str: &str, bin_data: &[u8]) -> Result<()> {
    let json_bytes = json_str.as_bytes();
    // Pad JSON to 4-byte alignment with spaces
    let json_pad = (4 - json_bytes.len() % 4) % 4;
    let json_chunk_len = json_bytes.len() + json_pad;

    // Pad BIN to 4-byte alignment with zeros
    let bin_pad = (4 - bin_data.len() % 4) % 4;
    let bin_chunk_len = bin_data.len() + bin_pad;

    // Total file length = 12 (header) + 8 (json chunk header) + json_chunk_len
    //                    + 8 (bin chunk header) + bin_chunk_len
    let total_len = 12 + 8 + json_chunk_len + if !bin_data.is_empty() { 8 + bin_chunk_len } else { 0 };

    let mut file = std::fs::File::create(path)?;

    // GLB Header
    file.write_all(b"glTF")?;                           // magic
    file.write_all(&2u32.to_le_bytes())?;                // version
    file.write_all(&(total_len as u32).to_le_bytes())?;  // length

    // JSON chunk
    file.write_all(&(json_chunk_len as u32).to_le_bytes())?; // chunk length
    file.write_all(&0x4E4F534Au32.to_le_bytes())?;           // chunk type (JSON)
    file.write_all(json_bytes)?;
    for _ in 0..json_pad {
        file.write_all(b" ")?;
    }

    // BIN chunk (only if we have data)
    if !bin_data.is_empty() {
        file.write_all(&(bin_chunk_len as u32).to_le_bytes())?; // chunk length
        file.write_all(&0x004E4942u32.to_le_bytes())?;          // chunk type (BIN)
        file.write_all(bin_data)?;
        for _ in 0..bin_pad {
            file.write_all(&[0u8])?;
        }
    }

    file.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_to_local_root() {
        let globals = vec![
            Mat4::from_translation(Vec3::new(1.0, 2.0, 3.0)),
        ];
        let parents = vec![-1i32];
        let locals = global_to_local(&globals, &parents);
        assert!((locals[0].0.x - 1.0).abs() < 0.01);
        assert!((locals[0].0.y - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_global_to_local_child() {
        let parent = Mat4::from_translation(Vec3::new(1.0, 0.0, 0.0));
        let child = Mat4::from_translation(Vec3::new(3.0, 0.0, 0.0));
        let globals = vec![parent, child];
        let parents = vec![-1i32, 0];
        let locals = global_to_local(&globals, &parents);
        // Child local should be (2, 0, 0) relative to parent
        assert!((locals[1].0.x - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_bin_writer_alignment() {
        let mut bw = BinWriter::new();
        bw.write_u8_slice(&[1, 2, 3]);
        assert_eq!(bw.offset(), 3);
        bw.align4();
        assert_eq!(bw.offset(), 4);
        bw.align4();
        assert_eq!(bw.offset(), 4); // already aligned
    }

    #[test]
    fn test_export_empty_model() {
        let model = ImportedModel {
            name: "test".into(),
            meshes: Vec::new(),
            skin: None,
            joint_names: vec!["Root".into()],
            parent_indices: vec![-1],
            animation_frames: None,
        };
        let dir = std::env::temp_dir();
        let path = dir.join("test_export.glb");
        let result = export_glb(&path, &model, None, 30.0);
        assert!(result.is_ok());
        // Verify file starts with glTF magic
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(&bytes[0..4], b"glTF");
        assert_eq!(u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]), 2);
        std::fs::remove_file(&path).ok();
    }
}
