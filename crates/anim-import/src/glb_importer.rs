//! GLB/glTF importer using the gltf crate.

use std::path::Path;
use glam::{Mat4, Vec3, Vec2, Quat};
use gltf::Document;
use anyhow::{Result, Context};
use crate::mesh::*;

pub struct GlbImporter;

impl GlbImporter {
    /// Load a GLB/glTF file and extract mesh, skeleton, and animation data.
    pub fn load(path: &Path) -> Result<ImportedModel> {
        let (document, buffers, _images) = gltf::import(path)
            .with_context(|| format!("Impossible de charger: {}", path.display()))?;

        let file_name = path.file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "model".to_string());

        // Collect all nodes into a flat array with parent indices
        let (node_names, node_parents, node_local_transforms) =
            Self::collect_nodes(&document);

        // Extract skin data
        let skin = Self::extract_skin(&document, &buffers, &node_names);

        // Extract meshes
        let meshes = Self::extract_meshes(&document, &buffers);

        // Extract animation
        let animation = Self::extract_animation(
            &document, &buffers, &node_names, &node_parents, &node_local_transforms,
        );

        // Determine joint names and parent indices from skin or nodes
        let (joint_names, parent_indices) = if let Some(ref s) = skin {
            let names = s.joint_names.clone();
            let parents = Self::compute_joint_parents(&s.joint_indices, &node_parents);
            (names, parents)
        } else {
            (node_names.clone(), node_parents.clone())
        };

        Ok(ImportedModel {
            name: file_name,
            meshes,
            skin,
            joint_names,
            parent_indices,
            animation_frames: animation,
        })
    }

    fn collect_nodes(document: &Document) -> (Vec<String>, Vec<i32>, Vec<Mat4>) {
        let mut names = Vec::new();
        let mut parents = Vec::new();
        let mut locals = Vec::new();

        fn visit(
            node: &gltf::Node,
            parent_idx: i32,
            names: &mut Vec<String>,
            parents: &mut Vec<i32>,
            locals: &mut Vec<Mat4>,
        ) {
            let _idx = names.len();
            let name = node.name().unwrap_or("Node").to_string();
            names.push(name);
            parents.push(parent_idx);

            let (t, r, s) = node.transform().decomposed();
            let translation = Vec3::from(t);
            let rotation = Quat::from_array(r);
            let scale = Vec3::from(s);
            locals.push(Mat4::from_scale_rotation_translation(scale, rotation, translation));

            let current_idx = (names.len() - 1) as i32;
            for child in node.children() {
                visit(&child, current_idx, names, parents, locals);
            }
        }

        for scene in document.scenes() {
            for node in scene.nodes() {
                visit(&node, -1, &mut names, &mut parents, &mut locals);
            }
        }

        (names, parents, locals)
    }

    fn extract_skin(
        document: &Document,
        buffers: &[gltf::buffer::Data],
        _node_names: &[String],
    ) -> Option<ImportedSkin> {
        let skin = document.skins().next()?;

        let joint_indices: Vec<usize> = skin.joints().map(|j| j.index()).collect();
        let joint_names: Vec<String> = skin.joints()
            .map(|j| j.name().unwrap_or("Joint").to_string())
            .collect();

        let reader = skin.reader(|buf| Some(&buffers[buf.index()]));
        let ibm: Vec<Mat4> = reader.read_inverse_bind_matrices()
            .map(|iter| {
                iter.map(|m| {
                    // m is [[f32; 4]; 4] column-major
                    Mat4::from_cols_array_2d(&m)
                }).collect()
            })
            .unwrap_or_else(|| vec![Mat4::IDENTITY; joint_indices.len()]);

        Some(ImportedSkin {
            inverse_bind_matrices: ibm,
            joint_names,
            joint_indices,
        })
    }

    fn extract_meshes(
        document: &Document,
        buffers: &[gltf::buffer::Data],
    ) -> Vec<ImportedMesh> {
        let mut meshes = Vec::new();

        for mesh in document.meshes() {
            for primitive in mesh.primitives() {
                let reader = primitive.reader(|buf| Some(&buffers[buf.index()]));

                let vertices: Vec<Vec3> = reader.read_positions()
                    .map(|iter| iter.map(Vec3::from).collect())
                    .unwrap_or_default();

                let normals: Vec<Vec3> = reader.read_normals()
                    .map(|iter| iter.map(Vec3::from).collect())
                    .unwrap_or_else(|| vec![Vec3::Y; vertices.len()]);

                let texcoords: Vec<Vec2> = reader.read_tex_coords(0)
                    .map(|tc| tc.into_f32().map(Vec2::from).collect())
                    .unwrap_or_else(|| vec![Vec2::ZERO; vertices.len()]);

                let indices: Vec<u32> = reader.read_indices()
                    .map(|idx| idx.into_u32().collect())
                    .unwrap_or_default();

                let bone_indices: Vec<[u32; 4]> = reader.read_joints(0)
                    .map(|j| j.into_u16().map(|js| [js[0] as u32, js[1] as u32, js[2] as u32, js[3] as u32]).collect())
                    .unwrap_or_else(|| vec![[0; 4]; vertices.len()]);

                let bone_weights: Vec<[f32; 4]> = reader.read_weights(0)
                    .map(|w| w.into_f32().collect())
                    .unwrap_or_else(|| vec![[1.0, 0.0, 0.0, 0.0]; vertices.len()]);

                meshes.push(ImportedMesh {
                    vertices,
                    normals,
                    texcoords,
                    indices,
                    bone_indices,
                    bone_weights,
                    texture: None,
                    normal_map: None,
                    metallic_roughness_map: None,
                    emission_map: None,
                    material_index: 0,
                });
            }
        }

        meshes
    }

    fn extract_animation(
        document: &Document,
        buffers: &[gltf::buffer::Data],
        node_names: &[String],
        node_parents: &[i32],
        node_local_transforms: &[Mat4],
    ) -> Option<AnimationData> {
        let animation = document.animations().next()?;
        let num_nodes = node_names.len();

        let mut max_frames = 0usize;
        let mut max_time = 0.0f32;

        // Collect per-node animation channels
        let mut translations: Vec<Option<Vec<Vec3>>> = vec![None; num_nodes];
        let mut rotations: Vec<Option<Vec<Quat>>> = vec![None; num_nodes];
        let mut scales: Vec<Option<Vec<Vec3>>> = vec![None; num_nodes];

        for channel in animation.channels() {
            let target_node = channel.target().node().index();
            if target_node >= num_nodes { continue; }

            let reader = channel.reader(|buf| Some(&buffers[buf.index()]));

            if let Some(timestamps) = reader.read_inputs() {
                let times: Vec<f32> = timestamps.collect();
                if let Some(&last) = times.last() {
                    if last > max_time { max_time = last; }
                }
                if times.len() > max_frames { max_frames = times.len(); }
            }

            match channel.target().property() {
                gltf::animation::Property::Translation => {
                    if let Some(output) = reader.read_outputs() {
                        if let gltf::animation::util::ReadOutputs::Translations(iter) = output {
                            translations[target_node] = Some(iter.map(Vec3::from).collect());
                        }
                    }
                }
                gltf::animation::Property::Rotation => {
                    if let Some(output) = reader.read_outputs() {
                        if let gltf::animation::util::ReadOutputs::Rotations(iter) = output {
                            rotations[target_node] = Some(
                                iter.into_f32().map(Quat::from_array).collect()
                            );
                        }
                    }
                }
                gltf::animation::Property::Scale => {
                    if let Some(output) = reader.read_outputs() {
                        if let gltf::animation::util::ReadOutputs::Scales(iter) = output {
                            scales[target_node] = Some(iter.map(Vec3::from).collect());
                        }
                    }
                }
                _ => {}
            }
        }

        if max_frames == 0 { return None; }

        let framerate = if max_time > 0.0 {
            (max_frames as f32 - 1.0) / max_time
        } else {
            30.0
        };

        // Build per-frame local transforms, then FK to globals
        let mut frames = Vec::with_capacity(max_frames);
        for frame_idx in 0..max_frames {
            let mut local_transforms = node_local_transforms.to_vec();

            for node_idx in 0..num_nodes {
                let t = translations[node_idx].as_ref()
                    .and_then(|ts| ts.get(frame_idx.min(ts.len().saturating_sub(1))).copied());
                let r = rotations[node_idx].as_ref()
                    .and_then(|rs| rs.get(frame_idx.min(rs.len().saturating_sub(1))).copied());
                let s = scales[node_idx].as_ref()
                    .and_then(|ss| ss.get(frame_idx.min(ss.len().saturating_sub(1))).copied());

                if t.is_some() || r.is_some() || s.is_some() {
                    let (def_s, def_r, def_t) = node_local_transforms[node_idx]
                        .to_scale_rotation_translation();
                    let trans = t.unwrap_or(def_t);
                    let rot = r.unwrap_or(def_r);
                    let scl = s.unwrap_or(def_s);
                    local_transforms[node_idx] = Mat4::from_scale_rotation_translation(scl, rot, trans);
                }
            }

            // FK
            let mut global = vec![Mat4::IDENTITY; num_nodes];
            for i in 0..num_nodes {
                let parent = node_parents[i];
                if parent < 0 {
                    global[i] = local_transforms[i];
                } else {
                    global[i] = global[parent as usize] * local_transforms[i];
                }
            }
            frames.push(global);
        }

        Some(AnimationData { frames, framerate })
    }

    fn compute_joint_parents(joint_indices: &[usize], node_parents: &[i32]) -> Vec<i32> {
        let joint_set: std::collections::HashSet<usize> = joint_indices.iter().copied().collect();
        let index_map: std::collections::HashMap<usize, usize> = joint_indices
            .iter()
            .enumerate()
            .map(|(local, &global)| (global, local))
            .collect();

        joint_indices
            .iter()
            .map(|&global_idx| {
                let mut current = node_parents[global_idx];
                while current >= 0 {
                    if joint_set.contains(&(current as usize)) {
                        return *index_map.get(&(current as usize)).unwrap() as i32;
                    }
                    current = node_parents[current as usize];
                }
                -1
            })
            .collect()
    }
}
