//! FBX file importer using fbxcel-dom.
//!
//! Supports FBX 7.4+ binary format. Extracts:
//! - Mesh geometry (vertices, normals, UVs, indices)
//! - Skeleton hierarchy (bones, parent relationships)
//! - Skin weights (bone influences per vertex)
//! - Animation curves (joint transforms per frame)

use std::collections::HashMap;
use std::io::BufReader;
use std::path::Path;
use anyhow::{Context, Result, bail};
use glam::{Mat4, Vec3, Vec2};
use crate::mesh::{ImportedModel, ImportedMesh, ImportedSkin, AnimationData};

pub struct FbxImporter;

impl FbxImporter {
    pub fn load(path: &Path) -> Result<ImportedModel> {
        let name = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("fbx_model")
            .to_string();

        let file = std::fs::File::open(path)
            .with_context(|| format!("Cannot open: {}", path.display()))?;
        let reader = BufReader::new(file);

        let doc = match fbxcel_dom::any::AnyDocument::from_seekable_reader(reader)
            .with_context(|| format!("FBX parse error: {}", path.display()))?
        {
            fbxcel_dom::any::AnyDocument::V7400(_ver, doc) => doc,
            _ => bail!("Unsupported FBX version (need 7.4+)"),
        };

        let mut builder = FbxBuilder::new(name);
        builder.extract(&doc)?;
        builder.build()
    }
}

/// Internal builder that accumulates FBX data.
struct FbxBuilder {
    name: String,
    // Skeleton
    bone_names: Vec<String>,
    bone_parents: Vec<i32>,
    bone_ids: Vec<i64>,          // FBX object IDs
    bone_id_to_idx: HashMap<i64, usize>,
    bone_bind_transforms: Vec<Mat4>,
    // Mesh
    vertices: Vec<Vec3>,
    normals: Vec<Vec3>,
    texcoords: Vec<Vec2>,
    indices: Vec<u32>,
    bone_indices_per_vert: Vec<[u32; 4]>,
    bone_weights_per_vert: Vec<[f32; 4]>,
    // Animation
    anim_frames: Vec<Vec<Mat4>>,
    framerate: f32,
}

impl FbxBuilder {
    fn new(name: String) -> Self {
        Self {
            name,
            bone_names: Vec::new(),
            bone_parents: Vec::new(),
            bone_ids: Vec::new(),
            bone_id_to_idx: HashMap::new(),
            bone_bind_transforms: Vec::new(),
            vertices: Vec::new(),
            normals: Vec::new(),
            texcoords: Vec::new(),
            indices: Vec::new(),
            bone_indices_per_vert: Vec::new(),
            bone_weights_per_vert: Vec::new(),
            anim_frames: Vec::new(),
            framerate: 30.0,
        }
    }

    fn extract(&mut self, doc: &fbxcel_dom::v7400::Document) -> Result<()> {
        use fbxcel_dom::v7400::object::TypedObjectHandle;

        // Pass 1: Collect all Model nodes (bones/nulls/meshes) and their FBX IDs
        let mut model_nodes: Vec<(i64, String, String)> = Vec::new(); // (id, name, class)
        let mut mesh_obj_ids: Vec<i64> = Vec::new();

        for obj in doc.objects() {
            let obj_id = obj.object_id().raw();
            match obj.get_typed() {
                TypedObjectHandle::Model(model) => {
                    let obj_name = obj.name().unwrap_or("").to_string();
                    let class = model.class().to_string();
                    model_nodes.push((obj_id, obj_name, class));
                }
                TypedObjectHandle::Geometry(_) => {
                    mesh_obj_ids.push(obj_id);
                }
                _ => {}
            }
        }

        // Pass 2: Build parent-child relationships from connections
        let mut parent_map: HashMap<i64, i64> = HashMap::new();
        // FBX connections: child -> parent (OO connections)
        let tree = doc.tree();
        if let Some(conns_node) = tree.root().first_child_by_name("Connections") {
            for child in conns_node.children() {
                if child.name() == "C" {
                    let attrs = child.attributes();
                    if attrs.len() >= 3 {
                        if let (Some(conn_type), Some(child_id), Some(parent_id)) = (
                            attr_as_str(&attrs[0]),
                            attr_as_i64(&attrs[1]),
                            attr_as_i64(&attrs[2]),
                        ) {
                            if conn_type == "OO" {
                                parent_map.insert(child_id, parent_id);
                            }
                        }
                    }
                }
            }
        }

        // Pass 3: Identify bones (LimbNode, Null, Root types) and build hierarchy
        let bone_classes = ["LimbNode", "Limb", "Root", "Null"];
        let mut all_model_ids: Vec<i64> = Vec::new();

        for (obj_id, obj_name, class) in &model_nodes {
            // Include if it's a bone-type or if there's no mesh (everything is a bone)
            if bone_classes.iter().any(|c| class.contains(c)) || mesh_obj_ids.is_empty() {
                let idx = self.bone_names.len();
                self.bone_names.push(clean_bone_name(obj_name));
                self.bone_ids.push(*obj_id);
                self.bone_id_to_idx.insert(*obj_id, idx);
                self.bone_parents.push(-1); // resolved later
                self.bone_bind_transforms.push(Mat4::IDENTITY);
                all_model_ids.push(*obj_id);
            }
        }

        // If no bones found via class filtering, try all Model nodes as bones
        if self.bone_names.is_empty() {
            for (obj_id, obj_name, _class) in &model_nodes {
                let idx = self.bone_names.len();
                self.bone_names.push(clean_bone_name(obj_name));
                self.bone_ids.push(*obj_id);
                self.bone_id_to_idx.insert(*obj_id, idx);
                self.bone_parents.push(-1);
                self.bone_bind_transforms.push(Mat4::IDENTITY);
            }
        }

        // Resolve parent indices
        for i in 0..self.bone_ids.len() {
            let bone_id = self.bone_ids[i];
            if let Some(&parent_id) = parent_map.get(&bone_id) {
                if let Some(&parent_idx) = self.bone_id_to_idx.get(&parent_id) {
                    self.bone_parents[i] = parent_idx as i32;
                }
            }
        }

        // Pass 4: Read local transforms for each bone from Properties70
        self.read_bone_transforms(doc)?;

        // Pass 5: Extract mesh geometry
        self.extract_meshes(doc)?;

        // Pass 6: Extract skin weights
        self.extract_skin(doc, &parent_map)?;

        // Pass 7: Extract animation
        self.extract_animation(doc)?;

        Ok(())
    }

    fn read_bone_transforms(&mut self, doc: &fbxcel_dom::v7400::Document) -> Result<()> {
        let tree = doc.tree();

        if let Some(objects_node) = tree.root().first_child_by_name("Objects") {
            for obj_node in objects_node.children() {
                if obj_node.name() != "Model" { continue; }

                let attrs = obj_node.attributes();
                let obj_id = attrs.first().and_then(|a| attr_as_i64(a)).unwrap_or(0);

                if let Some(&bone_idx) = self.bone_id_to_idx.get(&obj_id) {
                    let (t, r, s) = read_trs_from_properties70(obj_node);
                    self.bone_bind_transforms[bone_idx] = compose_trs(t, r, s);
                }
            }
        }

        Ok(())
    }

    fn extract_meshes(&mut self, doc: &fbxcel_dom::v7400::Document) -> Result<()> {
        let tree = doc.tree();

        if let Some(objects_node) = tree.root().first_child_by_name("Objects") {
            for obj_node in objects_node.children() {
                if obj_node.name() != "Geometry" { continue; }

                // Read vertices
                if let Some(verts_node) = obj_node.first_child_by_name("Vertices") {
                    let attrs = verts_node.attributes();
                    if let Some(data) = attrs.first().and_then(|a| attr_as_f64_array(a)) {
                        for chunk in data.chunks_exact(3) {
                            self.vertices.push(Vec3::new(
                                chunk[0] as f32,
                                chunk[1] as f32,
                                chunk[2] as f32,
                            ));
                        }
                    }
                }

                // Read polygon vertex indices
                if let Some(idx_node) = obj_node.first_child_by_name("PolygonVertexIndex") {
                    let attrs = idx_node.attributes();
                    if let Some(raw_indices) = attrs.first().and_then(|a| attr_as_i32_array(a)) {
                        // FBX encodes polygon end with bitwise NOT (~idx)
                        // Triangulate polygons on the fly
                        let mut polygon_verts: Vec<u32> = Vec::new();

                        for &idx in &raw_indices {
                            let actual_idx = if idx < 0 { !idx } else { idx } as u32;
                            polygon_verts.push(actual_idx);

                            if idx < 0 {
                                // End of polygon — triangulate as a fan
                                if polygon_verts.len() >= 3 {
                                    let v0 = polygon_verts[0];
                                    for i in 1..polygon_verts.len() - 1 {
                                        self.indices.push(v0);
                                        self.indices.push(polygon_verts[i]);
                                        self.indices.push(polygon_verts[i + 1]);
                                    }
                                }
                                polygon_verts.clear();
                            }
                        }
                    }
                }

                // Read normals from LayerElementNormal
                if let Some(normal_layer) = obj_node.first_child_by_name("LayerElementNormal") {
                    if let Some(normals_node) = normal_layer.first_child_by_name("Normals") {
                        let attrs = normals_node.attributes();
                        if let Some(data) = attrs.first().and_then(|a| attr_as_f64_array(a)) {
                            self.normals.clear();
                            for chunk in data.chunks_exact(3) {
                                self.normals.push(Vec3::new(
                                    chunk[0] as f32,
                                    chunk[1] as f32,
                                    chunk[2] as f32,
                                ));
                            }
                        }
                    }
                }

                // Read UVs from LayerElementUV
                if let Some(uv_layer) = obj_node.first_child_by_name("LayerElementUV") {
                    if let Some(uv_node) = uv_layer.first_child_by_name("UV") {
                        let attrs = uv_node.attributes();
                        if let Some(data) = attrs.first().and_then(|a| attr_as_f64_array(a)) {
                            self.texcoords.clear();
                            for chunk in data.chunks_exact(2) {
                                self.texcoords.push(Vec2::new(
                                    chunk[0] as f32,
                                    chunk[1] as f32,
                                ));
                            }
                        }
                    }
                }

                // Only process the first geometry
                break;
            }
        }

        // Generate normals if missing
        if self.normals.is_empty() && !self.vertices.is_empty() {
            self.normals.resize(self.vertices.len(), Vec3::Y);
        }
        // Generate UVs if missing
        if self.texcoords.is_empty() && !self.vertices.is_empty() {
            self.texcoords.resize(self.vertices.len(), Vec2::ZERO);
        }

        Ok(())
    }

    fn extract_skin(
        &mut self,
        doc: &fbxcel_dom::v7400::Document,
        parent_map: &HashMap<i64, i64>,
    ) -> Result<()> {
        if self.vertices.is_empty() { return Ok(()); }

        let vert_count = self.vertices.len();
        let mut weights: Vec<Vec<(u32, f32)>> = vec![Vec::new(); vert_count];

        let tree = doc.tree();
        if let Some(objects_node) = tree.root().first_child_by_name("Objects") {
            for obj_node in objects_node.children() {
                if obj_node.name() != "Deformer" { continue; }

                let attrs = obj_node.attributes();
                // SubDeformer (Cluster) has class "Cluster"
                let class = attrs.get(2).and_then(|a| attr_as_str(a)).unwrap_or("");
                if class != "Cluster" { continue; }

                let cluster_id = attrs.first().and_then(|a| attr_as_i64(a)).unwrap_or(0);

                // Find which bone this cluster maps to via connections
                let bone_idx = find_cluster_bone(cluster_id, parent_map, &self.bone_id_to_idx);

                if let Some(bone_idx) = bone_idx {
                    // Read Indexes (affected vertex indices)
                    let vert_indices = obj_node
                        .first_child_by_name("Indexes")
                        .and_then(|n| n.attributes().first().and_then(attr_as_i32_array))
                        .unwrap_or_default();

                    // Read Weights
                    let w = obj_node
                        .first_child_by_name("Weights")
                        .and_then(|n| n.attributes().first().and_then(attr_as_f64_array))
                        .unwrap_or_default();

                    for (i, &vi) in vert_indices.iter().enumerate() {
                        if (vi as usize) < vert_count && i < w.len() {
                            weights[vi as usize].push((bone_idx as u32, w[i] as f32));
                        }
                    }
                }
            }
        }

        // Convert to 4-bone-per-vertex format
        self.bone_indices_per_vert.resize(vert_count, [0; 4]);
        self.bone_weights_per_vert.resize(vert_count, [0.0; 4]);

        for (vi, w_list) in weights.iter_mut().enumerate() {
            // Sort by weight descending, take top 4
            w_list.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            let count = w_list.len().min(4);
            let mut total = 0.0f32;
            for i in 0..count {
                self.bone_indices_per_vert[vi][i] = w_list[i].0;
                self.bone_weights_per_vert[vi][i] = w_list[i].1;
                total += w_list[i].1;
            }
            // Normalize weights
            if total > 0.0 {
                for i in 0..count {
                    self.bone_weights_per_vert[vi][i] /= total;
                }
            }
        }

        Ok(())
    }

    fn extract_animation(&mut self, doc: &fbxcel_dom::v7400::Document) -> Result<()> {
        if self.bone_names.is_empty() { return Ok(()); }

        let tree = doc.tree();

        // Read global framerate from GlobalSettings
        if let Some(settings) = tree.root().first_child_by_name("GlobalSettings") {
            if let Some(props) = settings.first_child_by_name("Properties70") {
                for p in props.children() {
                    let attrs = p.attributes();
                    if let Some(name) = attrs.first().and_then(|a| attr_as_str(a)) {
                        if name == "CustomFrameRate" {
                            if let Some(val) = attrs.last().and_then(|a| attr_as_f64(a)) {
                                if val > 0.0 { self.framerate = val as f32; }
                            }
                        }
                    }
                }
            }
        }

        // Build animation curves: map from (bone_object_id, channel) -> curve data
        // Channel: "d|X", "d|Y", "d|Z" for T/R/S
        let mut curve_nodes: HashMap<i64, (i64, String)> = HashMap::new(); // curvenode_id -> (bone_id, property)
        let mut curves: HashMap<i64, (Vec<f64>, Vec<f64>)> = HashMap::new(); // curve_id -> (times, values)

        // Build connection map: child_id -> (parent_id, property_name)
        let mut conn_map: HashMap<i64, Vec<(i64, String)>> = HashMap::new();

        if let Some(conns_node) = tree.root().first_child_by_name("Connections") {
            for child in conns_node.children() {
                if child.name() != "C" { continue; }
                let attrs = child.attributes();
                if attrs.len() >= 3 {
                    let child_id = attr_as_i64(&attrs[1]).unwrap_or(0);
                    let parent_id = attr_as_i64(&attrs[2]).unwrap_or(0);
                    let prop = if attrs.len() >= 4 {
                        attr_as_str(&attrs[3]).unwrap_or("").to_string()
                    } else {
                        String::new()
                    };
                    conn_map.entry(child_id).or_default().push((parent_id, prop));
                }
            }
        }

        if let Some(objects_node) = tree.root().first_child_by_name("Objects") {
            // Collect AnimationCurveNode objects
            for obj_node in objects_node.children() {
                if obj_node.name() != "AnimationCurveNode" { continue; }
                let attrs = obj_node.attributes();
                let node_id = attrs.first().and_then(|a| attr_as_i64(a)).unwrap_or(0);

                // Find which bone this curve node connects to
                if let Some(conns) = conn_map.get(&node_id) {
                    for (parent_id, prop) in conns {
                        if self.bone_id_to_idx.contains_key(parent_id) {
                            let prop_name = if !prop.is_empty() {
                                prop.clone()
                            } else {
                                attrs.get(1).and_then(|a| attr_as_str(a)).unwrap_or("").to_string()
                            };
                            curve_nodes.insert(node_id, (*parent_id, prop_name));
                        }
                    }
                }
            }

            // Collect AnimationCurve objects
            for obj_node in objects_node.children() {
                if obj_node.name() != "AnimationCurve" { continue; }
                let attrs = obj_node.attributes();
                let curve_id = attrs.first().and_then(|a| attr_as_i64(a)).unwrap_or(0);

                let times = obj_node.first_child_by_name("KeyTime")
                    .and_then(|n| n.attributes().first().and_then(attr_as_i64_array))
                    .unwrap_or_default()
                    .iter()
                    .map(|&t| t as f64 / 46186158000.0) // FBX time units to seconds
                    .collect::<Vec<f64>>();

                let values = obj_node.first_child_by_name("KeyValueFloat")
                    .and_then(|n| n.attributes().first().and_then(attr_as_f32_array))
                    .unwrap_or_default()
                    .iter()
                    .map(|&v| v as f64)
                    .collect::<Vec<f64>>();

                if !times.is_empty() && times.len() == values.len() {
                    curves.insert(curve_id, (times, values));
                }
            }
        }

        // If no animation curves found, compute bind pose as single frame
        if curves.is_empty() {
            let global_transforms = self.compute_global_bind_pose();
            if !global_transforms.is_empty() {
                self.anim_frames.push(global_transforms);
            }
            return Ok(());
        }

        // Find total time range
        let mut max_time: f64 = 0.0;
        for (_id, (times, _vals)) in &curves {
            if let Some(&t) = times.last() {
                if t > max_time { max_time = t; }
            }
        }

        if max_time <= 0.0 {
            let global_transforms = self.compute_global_bind_pose();
            if !global_transforms.is_empty() {
                self.anim_frames.push(global_transforms);
            }
            return Ok(());
        }

        // Build per-bone animation channels
        // bone_idx -> { "Lcl Translation" -> {x_curve, y_curve, z_curve}, ... }
        struct BoneAnim {
            tx: Option<(Vec<f64>, Vec<f64>)>,
            ty: Option<(Vec<f64>, Vec<f64>)>,
            tz: Option<(Vec<f64>, Vec<f64>)>,
            rx: Option<(Vec<f64>, Vec<f64>)>,
            ry: Option<(Vec<f64>, Vec<f64>)>,
            rz: Option<(Vec<f64>, Vec<f64>)>,
        }
        impl BoneAnim { fn new() -> Self { Self { tx: None, ty: None, tz: None, rx: None, ry: None, rz: None } } }

        let mut bone_anims: HashMap<usize, BoneAnim> = HashMap::new();

        // Map curves to bone channels
        for (curve_id, (times, values)) in &curves {
            // Find which curve node this curve connects to
            if let Some(conns) = conn_map.get(curve_id) {
                for (cn_id, channel) in conns {
                    if let Some((bone_obj_id, prop_name)) = curve_nodes.get(cn_id) {
                        if let Some(&bone_idx) = self.bone_id_to_idx.get(bone_obj_id) {
                            let ba = bone_anims.entry(bone_idx).or_insert_with(BoneAnim::new);
                            let data = Some((times.clone(), values.clone()));
                            let is_translation = prop_name.contains("Translation") || prop_name.contains("T");
                            let is_rotation = prop_name.contains("Rotation") || prop_name.contains("R");

                            match (channel.as_str(), is_translation, is_rotation) {
                                ("d|X", true, _) => ba.tx = data,
                                ("d|Y", true, _) => ba.ty = data,
                                ("d|Z", true, _) => ba.tz = data,
                                ("d|X", _, true) => ba.rx = data,
                                ("d|Y", _, true) => ba.ry = data,
                                ("d|Z", _, true) => ba.rz = data,
                                _ => {}
                            }
                        }
                    }
                }
            }
        }

        // Sample animation at regular intervals
        let dt = 1.0 / self.framerate as f64;
        let num_frames = ((max_time / dt).ceil() as usize).max(1);

        for frame in 0..num_frames {
            let t = frame as f64 * dt;
            let mut local_transforms: Vec<Mat4> = self.bone_bind_transforms.clone();

            for (&bone_idx, ba) in &bone_anims {
                if bone_idx >= local_transforms.len() { continue; }

                let base = self.bone_bind_transforms[bone_idx];
                let (base_t, base_r, base_s) = decompose_trs(base);

                let tx = sample_curve(&ba.tx, t, base_t.x as f64) as f32;
                let ty = sample_curve(&ba.ty, t, base_t.y as f64) as f32;
                let tz = sample_curve(&ba.tz, t, base_t.z as f64) as f32;

                let rx = sample_curve(&ba.rx, t, base_r.x as f64) as f32;
                let ry = sample_curve(&ba.ry, t, base_r.y as f64) as f32;
                let rz = sample_curve(&ba.rz, t, base_r.z as f64) as f32;

                local_transforms[bone_idx] = compose_trs(
                    Vec3::new(tx, ty, tz),
                    Vec3::new(rx, ry, rz),
                    base_s,
                );
            }

            // Convert local transforms to global
            let globals = self.local_to_global(&local_transforms);
            self.anim_frames.push(globals);
        }

        Ok(())
    }

    /// Compute global bind pose from local transforms.
    fn compute_global_bind_pose(&self) -> Vec<Mat4> {
        self.local_to_global(&self.bone_bind_transforms)
    }

    /// Convert local transforms to global using parent hierarchy.
    fn local_to_global(&self, local: &[Mat4]) -> Vec<Mat4> {
        let mut global = vec![Mat4::IDENTITY; local.len()];
        for i in 0..local.len() {
            let parent = self.bone_parents[i];
            if parent >= 0 && (parent as usize) < global.len() {
                global[i] = global[parent as usize] * local[i];
            } else {
                global[i] = local[i];
            }
        }
        global
    }

    fn build(self) -> Result<ImportedModel> {
        let has_mesh = !self.vertices.is_empty();
        let has_skin = has_mesh && !self.bone_weights_per_vert.is_empty();

        // Build skin first (before moving mesh data)
        let skin = if has_skin {
            let global_bind = local_to_global_static(&self.bone_bind_transforms, &self.bone_parents);
            let inverse_bind: Vec<Mat4> = global_bind.iter()
                .map(|m| m.inverse())
                .collect();
            Some(ImportedSkin {
                inverse_bind_matrices: inverse_bind,
                joint_names: self.bone_names.clone(),
                joint_indices: (0..self.bone_names.len()).collect(),
            })
        } else {
            None
        };

        let num_joints = self.bone_names.len();
        let vert_count = self.vertices.len();

        let meshes = if has_mesh {
            vec![ImportedMesh {
                vertices: self.vertices,
                normals: self.normals,
                texcoords: self.texcoords,
                indices: self.indices,
                bone_indices: self.bone_indices_per_vert,
                bone_weights: self.bone_weights_per_vert,
                texture: None,
            }]
        } else {
            Vec::new()
        };

        let animation_frames = if !self.anim_frames.is_empty() {
            Some(AnimationData {
                frames: self.anim_frames,
                framerate: self.framerate,
            })
        } else {
            None
        };

        let num_frames = animation_frames.as_ref().map_or(0, |a| a.frames.len());

        log::info!(
            "FBX: {} ({} joints, {} frames, {:.0} fps, {} vertices)",
            self.name, num_joints, num_frames, self.framerate, vert_count
        );

        Ok(ImportedModel {
            name: self.name,
            meshes,
            skin,
            joint_names: self.bone_names,
            parent_indices: self.bone_parents,
            animation_frames,
        })
    }
}

// ═══════════════════════════════════════════════════════════════
// FBX attribute helpers
// ═══════════════════════════════════════════════════════════════

use fbxcel::low::v7400::AttributeValue;
use fbxcel::tree::v7400::NodeHandle;

fn local_to_global_static(local: &[Mat4], parents: &[i32]) -> Vec<Mat4> {
    let mut global = vec![Mat4::IDENTITY; local.len()];
    for i in 0..local.len() {
        let parent = parents[i];
        if parent >= 0 && (parent as usize) < global.len() {
            global[i] = global[parent as usize] * local[i];
        } else {
            global[i] = local[i];
        }
    }
    global
}

fn attr_as_str(attr: &AttributeValue) -> Option<&str> {
    match attr {
        AttributeValue::String(s) => Some(s.as_str()),
        _ => None,
    }
}

fn attr_as_i64(attr: &AttributeValue) -> Option<i64> {
    match attr {
        AttributeValue::I64(v) => Some(*v),
        AttributeValue::I32(v) => Some(*v as i64),
        AttributeValue::I16(v) => Some(*v as i64),
        _ => None,
    }
}

fn attr_as_f64(attr: &AttributeValue) -> Option<f64> {
    match attr {
        AttributeValue::F64(v) => Some(*v),
        AttributeValue::F32(v) => Some(*v as f64),
        AttributeValue::I64(v) => Some(*v as f64),
        AttributeValue::I32(v) => Some(*v as f64),
        _ => None,
    }
}

fn attr_as_f64_array(attr: &AttributeValue) -> Option<Vec<f64>> {
    match attr {
        AttributeValue::ArrF64(arr) => Some(arr.to_vec()),
        AttributeValue::ArrF32(arr) => Some(arr.iter().map(|&v| v as f64).collect()),
        _ => None,
    }
}

fn attr_as_f32_array(attr: &AttributeValue) -> Option<Vec<f32>> {
    match attr {
        AttributeValue::ArrF32(arr) => Some(arr.to_vec()),
        AttributeValue::ArrF64(arr) => Some(arr.iter().map(|&v| v as f32).collect()),
        _ => None,
    }
}

fn attr_as_i32_array(attr: &AttributeValue) -> Option<Vec<i32>> {
    match attr {
        AttributeValue::ArrI32(arr) => Some(arr.to_vec()),
        AttributeValue::ArrI64(arr) => Some(arr.iter().map(|&v| v as i32).collect()),
        _ => None,
    }
}

fn attr_as_i64_array(attr: &AttributeValue) -> Option<Vec<i64>> {
    match attr {
        AttributeValue::ArrI64(arr) => Some(arr.to_vec()),
        AttributeValue::ArrI32(arr) => Some(arr.iter().map(|&v| v as i64).collect()),
        _ => None,
    }
}

// ═══════════════════════════════════════════════════════════════
// FBX transform helpers
// ═══════════════════════════════════════════════════════════════

/// Read Translation, Rotation, Scaling from a Properties70 block.
fn read_trs_from_properties70(node: NodeHandle<'_>) -> (Vec3, Vec3, Vec3) {
    let mut translation = Vec3::ZERO;
    let mut rotation = Vec3::ZERO;
    let mut scaling = Vec3::ONE;

    if let Some(props) = node.first_child_by_name("Properties70") {
        for p in props.children() {
            let attrs = p.attributes();
            if attrs.len() < 5 { continue; }
            let name = match attr_as_str(&attrs[0]) {
                Some(n) => n,
                None => continue,
            };

            match name {
                "Lcl Translation" => {
                    translation = Vec3::new(
                        attr_as_f64(&attrs[4]).unwrap_or(0.0) as f32,
                        attr_as_f64(&attrs[5]).unwrap_or(0.0) as f32,
                        attr_as_f64(&attrs[6]).unwrap_or(0.0) as f32,
                    );
                }
                "Lcl Rotation" => {
                    rotation = Vec3::new(
                        attr_as_f64(&attrs[4]).unwrap_or(0.0) as f32,
                        attr_as_f64(&attrs[5]).unwrap_or(0.0) as f32,
                        attr_as_f64(&attrs[6]).unwrap_or(0.0) as f32,
                    );
                }
                "Lcl Scaling" => {
                    scaling = Vec3::new(
                        attr_as_f64(&attrs[4]).unwrap_or(1.0) as f32,
                        attr_as_f64(&attrs[5]).unwrap_or(1.0) as f32,
                        attr_as_f64(&attrs[6]).unwrap_or(1.0) as f32,
                    );
                }
                _ => {}
            }
        }
    }

    (translation, rotation, scaling)
}

/// Compose a TRS matrix from translation (Vec3), rotation (Euler degrees XYZ), scale (Vec3).
fn compose_trs(translation: Vec3, rotation_deg: Vec3, scale: Vec3) -> Mat4 {
    let rx = rotation_deg.x.to_radians();
    let ry = rotation_deg.y.to_radians();
    let rz = rotation_deg.z.to_radians();

    let quat = glam::Quat::from_euler(glam::EulerRot::XYZ, rx, ry, rz);
    Mat4::from_scale_rotation_translation(scale, quat, translation)
}

/// Decompose a TRS matrix back into (translation, rotation_degrees, scale).
fn decompose_trs(mat: Mat4) -> (Vec3, Vec3, Vec3) {
    let (scale, rotation, translation) = mat.to_scale_rotation_translation();
    let (rx, ry, rz) = rotation.to_euler(glam::EulerRot::XYZ);
    (
        translation,
        Vec3::new(rx.to_degrees(), ry.to_degrees(), rz.to_degrees()),
        scale,
    )
}

/// Find the bone index connected to a skin cluster via connections.
fn find_cluster_bone(
    cluster_id: i64,
    parent_map: &HashMap<i64, i64>,
    bone_id_to_idx: &HashMap<i64, usize>,
) -> Option<usize> {
    // Clusters connect TO bones (reverse direction: bone -> cluster in some FBX,
    // or cluster -> bone in others). Check both directions.
    // In standard FBX, a Cluster's source connection points to a Model (bone).
    // parent_map is child->parent, so we need to check what connects TO this cluster.

    // The cluster connects to a bone via OO connection where the bone is a source
    // Since our parent_map is child->parent, look for entries where parent == cluster_id
    // These would be bones connecting to this cluster... but that's the wrong direction.

    // Actually, in FBX connections, the Model (bone) is connected as a child of the Cluster
    // So we need to look for entries in parent_map where parent_id == cluster_id
    // But our parent_map is child_id -> parent_id.
    // We need to invert: find child_ids whose parent is this cluster.

    // Build a quick reverse lookup
    for (&child_id, &p_id) in parent_map {
        if p_id == cluster_id {
            if let Some(&idx) = bone_id_to_idx.get(&child_id) {
                return Some(idx);
            }
        }
    }

    // Also check the other direction (cluster -> bone)
    if let Some(&parent_id) = parent_map.get(&cluster_id) {
        if let Some(&idx) = bone_id_to_idx.get(&parent_id) {
            return Some(idx);
        }
    }

    None
}

/// Sample an animation curve at time t (linear interpolation).
fn sample_curve(curve: &Option<(Vec<f64>, Vec<f64>)>, t: f64, default: f64) -> f64 {
    let (times, values) = match curve {
        Some(c) => c,
        None => return default,
    };

    if times.is_empty() { return default; }
    if t <= times[0] { return values[0]; }
    if t >= *times.last().unwrap() { return *values.last().unwrap(); }

    // Binary search for interval
    let idx = match times.binary_search_by(|probe| probe.partial_cmp(&t).unwrap_or(std::cmp::Ordering::Equal)) {
        Ok(i) => return values[i],
        Err(i) => i,
    };

    if idx == 0 { return values[0]; }
    let i = idx - 1;
    let frac = (t - times[i]) / (times[i + 1] - times[i]);
    values[i] + (values[i + 1] - values[i]) * frac
}

/// Clean FBX bone names (remove "Model::" prefix, null bytes, etc.).
fn clean_bone_name(name: &str) -> String {
    let cleaned = name
        .trim_start_matches("Model::")
        .trim_start_matches("Geometry::")
        .trim()
        .replace('\0', "");
    if cleaned.is_empty() { "Bone".to_string() } else { cleaned }
}
