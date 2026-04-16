//! Blend Tree: hierarchical animation blending with 1D/2D blend spaces.
//!
//! A blend tree evaluates a tree of nodes to produce a final blended pose.
//! Leaf nodes reference motion clips; interior nodes blend their children
//! based on named float parameters.

use glam::Mat4;
use std::collections::HashMap;
use crate::blend::blend_poses;

/// A complete blend tree with named parameters.
pub struct BlendTree {
    pub name: String,
    pub nodes: Vec<BlendTreeNode>,
    /// Root node index.
    pub root: usize,
    /// Named float parameters that drive blending.
    pub parameters: HashMap<String, f32>,
}

/// A node in the blend tree.
#[derive(Clone)]
pub enum BlendTreeNode {
    /// Leaf: plays a single motion clip.
    Clip(ClipNode),
    /// 1D blend space: blends N children along one parameter axis.
    Blend1D(Blend1DNode),
    /// 2D blend space: blends N children in a 2D parameter plane.
    Blend2D(Blend2DNode),
    /// Direct blend of exactly two children with a parameter-driven weight.
    Lerp(LerpNode),
}

/// Leaf node referencing a motion clip.
#[derive(Clone)]
pub struct ClipNode {
    pub name: String,
    /// Index into the loaded_models array (the asset that holds this motion).
    pub model_index: usize,
    /// Playback speed multiplier.
    pub speed: f32,
    /// Visual position in the editor.
    pub position: [f32; 2],
}

/// 1D blend space: children placed along a single parameter axis.
/// The parameter interpolates between the nearest two children.
#[derive(Clone)]
pub struct Blend1DNode {
    pub name: String,
    /// Parameter name (looked up in BlendTree.parameters).
    pub parameter: String,
    /// Children: (threshold_value, node_index).
    /// Must be sorted by threshold.
    pub children: Vec<(f32, usize)>,
    /// Visual position in the editor.
    pub position: [f32; 2],
}

/// 2D blend space: children placed on a 2D plane defined by two parameters.
/// Uses barycentric interpolation of the nearest triangle.
#[derive(Clone)]
pub struct Blend2DNode {
    pub name: String,
    /// X-axis parameter name.
    pub param_x: String,
    /// Y-axis parameter name.
    pub param_y: String,
    /// Children: (x_pos, y_pos, node_index).
    pub children: Vec<(f32, f32, usize)>,
    /// Visual position in the editor.
    pub position: [f32; 2],
}

/// Simple lerp between two children driven by a parameter (0..1).
#[derive(Clone)]
pub struct LerpNode {
    pub name: String,
    /// Parameter name (0.0 = child_a, 1.0 = child_b).
    pub parameter: String,
    pub child_a: usize,
    pub child_b: usize,
    /// Visual position in the editor.
    pub position: [f32; 2],
}

/// Result of evaluating a blend tree node.
pub struct BlendResult {
    pub pose: Vec<Mat4>,
    /// Effective playback speed (weighted average of contributing clips).
    pub speed: f32,
}

impl BlendTree {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            nodes: Vec::new(),
            root: 0,
            parameters: HashMap::new(),
        }
    }

    /// Add a node and return its index.
    pub fn add_node(&mut self, node: BlendTreeNode) -> usize {
        let idx = self.nodes.len();
        self.nodes.push(node);
        idx
    }

    /// Set a parameter value.
    pub fn set_parameter(&mut self, name: &str, value: f32) {
        self.parameters.insert(name.to_string(), value);
    }

    /// Get a parameter value.
    pub fn get_parameter(&self, name: &str) -> f32 {
        self.parameters.get(name).copied().unwrap_or(0.0)
    }

    /// Evaluate the tree and produce the final blended pose.
    /// `get_pose` is a closure that returns the current pose for a model_index.
    pub fn evaluate<F>(&self, get_pose: &F) -> Option<BlendResult>
    where
        F: Fn(usize) -> Option<Vec<Mat4>>,
    {
        if self.nodes.is_empty() {
            return None;
        }
        self.evaluate_node(self.root, get_pose)
    }

    fn evaluate_node<F>(&self, node_idx: usize, get_pose: &F) -> Option<BlendResult>
    where
        F: Fn(usize) -> Option<Vec<Mat4>>,
    {
        let node = self.nodes.get(node_idx)?;

        match node {
            BlendTreeNode::Clip(clip) => {
                let pose = get_pose(clip.model_index)?;
                Some(BlendResult { pose, speed: clip.speed })
            }

            BlendTreeNode::Blend1D(blend) => {
                self.evaluate_blend1d(blend, get_pose)
            }

            BlendTreeNode::Blend2D(blend) => {
                self.evaluate_blend2d(blend, get_pose)
            }

            BlendTreeNode::Lerp(lerp) => {
                let a = self.evaluate_node(lerp.child_a, get_pose)?;
                let b = self.evaluate_node(lerp.child_b, get_pose)?;
                let w = self.get_parameter(&lerp.parameter).clamp(0.0, 1.0);
                let pose = blend_poses(&a.pose, &b.pose, w);
                let speed = a.speed * (1.0 - w) + b.speed * w;
                Some(BlendResult { pose, speed })
            }
        }
    }

    fn evaluate_blend1d<F>(&self, blend: &Blend1DNode, get_pose: &F) -> Option<BlendResult>
    where
        F: Fn(usize) -> Option<Vec<Mat4>>,
    {
        if blend.children.is_empty() {
            return None;
        }
        if blend.children.len() == 1 {
            return self.evaluate_node(blend.children[0].1, get_pose);
        }

        let param = self.get_parameter(&blend.parameter);

        // Find the two surrounding children
        // Children must be sorted by threshold
        let mut lower = 0;
        let mut upper = blend.children.len() - 1;

        for (i, &(threshold, _)) in blend.children.iter().enumerate() {
            if threshold <= param {
                lower = i;
            }
            if threshold >= param && i < upper {
                upper = i;
                break;
            }
        }

        if lower == upper {
            return self.evaluate_node(blend.children[lower].1, get_pose);
        }

        let (t_low, idx_low) = blend.children[lower];
        let (t_high, idx_high) = blend.children[upper];

        let a = self.evaluate_node(idx_low, get_pose)?;
        let b = self.evaluate_node(idx_high, get_pose)?;

        let range = t_high - t_low;
        let w = if range.abs() < 1e-6 { 0.5 } else {
            ((param - t_low) / range).clamp(0.0, 1.0)
        };

        let pose = blend_poses(&a.pose, &b.pose, w);
        let speed = a.speed * (1.0 - w) + b.speed * w;
        Some(BlendResult { pose, speed })
    }

    fn evaluate_blend2d<F>(&self, blend: &Blend2DNode, get_pose: &F) -> Option<BlendResult>
    where
        F: Fn(usize) -> Option<Vec<Mat4>>,
    {
        if blend.children.is_empty() {
            return None;
        }
        if blend.children.len() == 1 {
            return self.evaluate_node(blend.children[0].2, get_pose);
        }

        let px = self.get_parameter(&blend.param_x);
        let py = self.get_parameter(&blend.param_y);

        // Find the 3 nearest children and use barycentric interpolation
        // For simplicity with small N, use inverse-distance weighting
        let mut weights: Vec<(usize, f32)> = Vec::new();
        let mut total_weight = 0.0f32;

        for &(cx, cy, node_idx) in &blend.children {
            let dx = px - cx;
            let dy = py - cy;
            let dist = (dx * dx + dy * dy).sqrt().max(1e-4);
            // Inverse distance squared for sharper falloff
            let w = 1.0 / (dist * dist);
            weights.push((node_idx, w));
            total_weight += w;
        }

        if total_weight < 1e-6 {
            return self.evaluate_node(blend.children[0].2, get_pose);
        }

        // Normalize weights
        for w in &mut weights {
            w.1 /= total_weight;
        }

        // Evaluate all children and blend
        // Multi-blend: accumulate progressively
        let mut evaluated: Vec<(Vec<Mat4>, f32, f32)> = Vec::new();
        for &(node_idx, w) in &weights {
            if w < 0.001 { continue; }
            if let Some(child) = self.evaluate_node(node_idx, get_pose) {
                evaluated.push((child.pose, child.speed, w));
            }
        }

        if evaluated.is_empty() {
            return None;
        }

        let mut final_pose = evaluated[0].0.clone();
        let mut final_speed = evaluated[0].2 * evaluated[0].1;
        let mut accumulated_weight = evaluated[0].2;

        for i in 1..evaluated.len() {
            let (ref pose, speed, w) = evaluated[i];
            accumulated_weight += w;
            let blend_w = w / accumulated_weight;
            final_pose = blend_poses(&final_pose, pose, blend_w);
            final_speed += w * speed;
        }

        Some(BlendResult { pose: final_pose, speed: final_speed })
    }

    /// Number of nodes.
    pub fn num_nodes(&self) -> usize {
        self.nodes.len()
    }

    /// Get all parameter names used by the tree.
    pub fn used_parameters(&self) -> Vec<String> {
        let mut params = Vec::new();
        for node in &self.nodes {
            match node {
                BlendTreeNode::Blend1D(b) => {
                    if !params.contains(&b.parameter) { params.push(b.parameter.clone()); }
                }
                BlendTreeNode::Blend2D(b) => {
                    if !params.contains(&b.param_x) { params.push(b.param_x.clone()); }
                    if !params.contains(&b.param_y) { params.push(b.param_y.clone()); }
                }
                BlendTreeNode::Lerp(l) => {
                    if !params.contains(&l.parameter) { params.push(l.parameter.clone()); }
                }
                _ => {}
            }
        }
        params
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;
    use anim_math::transform::Transform;

    fn make_pose(x: f32) -> Vec<Mat4> {
        vec![Mat4::from_translation(Vec3::new(x, 0.0, 0.0)); 3]
    }

    #[test]
    fn test_clip_node() {
        let mut tree = BlendTree::new("test");
        let clip = tree.add_node(BlendTreeNode::Clip(ClipNode {
            name: "walk".into(),
            model_index: 0,
            speed: 1.0,
            position: [0.0, 0.0],
        }));
        tree.root = clip;

        let result = tree.evaluate(&|idx| {
            if idx == 0 { Some(make_pose(1.0)) } else { None }
        });

        assert!(result.is_some());
        let r = result.unwrap();
        assert!((r.pose[0].get_position().x - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_blend1d() {
        let mut tree = BlendTree::new("test");
        let idle = tree.add_node(BlendTreeNode::Clip(ClipNode {
            name: "idle".into(), model_index: 0, speed: 1.0, position: [0.0, 0.0],
        }));
        let walk = tree.add_node(BlendTreeNode::Clip(ClipNode {
            name: "walk".into(), model_index: 1, speed: 1.0, position: [0.0, 0.0],
        }));
        let run = tree.add_node(BlendTreeNode::Clip(ClipNode {
            name: "run".into(), model_index: 2, speed: 1.5, position: [0.0, 0.0],
        }));
        let blend = tree.add_node(BlendTreeNode::Blend1D(Blend1DNode {
            name: "locomotion".into(),
            parameter: "speed".into(),
            children: vec![(0.0, idle), (1.0, walk), (3.0, run)],
            position: [0.0, 0.0],
        }));
        tree.root = blend;

        let get_pose = |idx: usize| -> Option<Vec<Mat4>> {
            match idx {
                0 => Some(make_pose(0.0)),  // idle at x=0
                1 => Some(make_pose(1.0)),  // walk at x=1
                2 => Some(make_pose(3.0)),  // run at x=3
                _ => None,
            }
        };

        // At speed=0 → fully idle (x=0)
        tree.set_parameter("speed", 0.0);
        let r = tree.evaluate(&get_pose).unwrap();
        assert!(r.pose[0].get_position().x.abs() < 0.01);

        // At speed=1 → fully walk (x=1)
        tree.set_parameter("speed", 1.0);
        let r = tree.evaluate(&get_pose).unwrap();
        assert!((r.pose[0].get_position().x - 1.0).abs() < 0.01);

        // At speed=0.5 → midpoint idle/walk (x=0.5)
        tree.set_parameter("speed", 0.5);
        let r = tree.evaluate(&get_pose).unwrap();
        assert!((r.pose[0].get_position().x - 0.5).abs() < 0.1);

        // At speed=2 → midpoint walk/run (x=2)
        tree.set_parameter("speed", 2.0);
        let r = tree.evaluate(&get_pose).unwrap();
        assert!((r.pose[0].get_position().x - 2.0).abs() < 0.1);
    }

    #[test]
    fn test_lerp_node() {
        let mut tree = BlendTree::new("test");
        let a = tree.add_node(BlendTreeNode::Clip(ClipNode {
            name: "a".into(), model_index: 0, speed: 1.0, position: [0.0, 0.0],
        }));
        let b = tree.add_node(BlendTreeNode::Clip(ClipNode {
            name: "b".into(), model_index: 1, speed: 2.0, position: [0.0, 0.0],
        }));
        let lerp = tree.add_node(BlendTreeNode::Lerp(LerpNode {
            name: "mix".into(),
            parameter: "mix".into(),
            child_a: a,
            child_b: b,
            position: [0.0, 0.0],
        }));
        tree.root = lerp;

        let get_pose = |idx: usize| -> Option<Vec<Mat4>> {
            match idx {
                0 => Some(make_pose(0.0)),
                1 => Some(make_pose(10.0)),
                _ => None,
            }
        };

        tree.set_parameter("mix", 0.0);
        let r = tree.evaluate(&get_pose).unwrap();
        assert!(r.pose[0].get_position().x.abs() < 0.01);
        assert!((r.speed - 1.0).abs() < 0.01);

        tree.set_parameter("mix", 1.0);
        let r = tree.evaluate(&get_pose).unwrap();
        assert!((r.pose[0].get_position().x - 10.0).abs() < 0.01);
        assert!((r.speed - 2.0).abs() < 0.01);

        tree.set_parameter("mix", 0.5);
        let r = tree.evaluate(&get_pose).unwrap();
        assert!((r.pose[0].get_position().x - 5.0).abs() < 0.1);
    }

    #[test]
    fn test_blend2d_basic() {
        let mut tree = BlendTree::new("test");
        let n = tree.add_node(BlendTreeNode::Clip(ClipNode {
            name: "north".into(), model_index: 0, speed: 1.0, position: [0.0, 0.0],
        }));
        let s = tree.add_node(BlendTreeNode::Clip(ClipNode {
            name: "south".into(), model_index: 1, speed: 1.0, position: [0.0, 0.0],
        }));
        let blend2d = tree.add_node(BlendTreeNode::Blend2D(Blend2DNode {
            name: "dir".into(),
            param_x: "dx".into(),
            param_y: "dy".into(),
            children: vec![(0.0, 1.0, n), (0.0, -1.0, s)],
            position: [0.0, 0.0],
        }));
        tree.root = blend2d;

        let get_pose = |idx: usize| -> Option<Vec<Mat4>> {
            match idx {
                0 => Some(make_pose(1.0)),   // north
                1 => Some(make_pose(-1.0)),  // south
                _ => None,
            }
        };

        // Close to north → should be near x=1
        tree.set_parameter("dx", 0.0);
        tree.set_parameter("dy", 0.9);
        let r = tree.evaluate(&get_pose).unwrap();
        assert!(r.pose[0].get_position().x > 0.5);
    }

    #[test]
    fn test_used_parameters() {
        let mut tree = BlendTree::new("test");
        tree.add_node(BlendTreeNode::Blend1D(Blend1DNode {
            name: "a".into(), parameter: "speed".into(), children: vec![], position: [0.0, 0.0],
        }));
        tree.add_node(BlendTreeNode::Blend2D(Blend2DNode {
            name: "b".into(), param_x: "dx".into(), param_y: "dy".into(),
            children: vec![], position: [0.0, 0.0],
        }));
        tree.add_node(BlendTreeNode::Lerp(LerpNode {
            name: "c".into(), parameter: "mix".into(),
            child_a: 0, child_b: 1, position: [0.0, 0.0],
        }));
        let params = tree.used_parameters();
        assert!(params.contains(&"speed".to_string()));
        assert!(params.contains(&"dx".to_string()));
        assert!(params.contains(&"dy".to_string()));
        assert!(params.contains(&"mix".to_string()));
    }
}
