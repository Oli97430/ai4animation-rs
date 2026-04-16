//! Built-in skeleton presets — ready-to-use humanoid, quadruped, hand, and spider rigs.
//!
//! These create ImportedModel instances without any file I/O, providing instant
//! skeleton assets for testing, learning, and prototyping animations.

use glam::{Mat4, Vec3};
use crate::ImportedModel;
use crate::mesh::AnimationData;

/// Available skeleton presets.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SkeletonPreset {
    /// Full humanoid (65 joints): hips, spine, head, 2 arms with fingers, 2 legs with toes
    Humanoid,
    /// Simple humanoid (19 joints): hips, spine, head, 2 arms, 2 legs — no fingers
    HumanoidSimple,
    /// Quadruped animal (25 joints): body, 4 legs, tail, head
    Quadruped,
    /// Hand with fingers (21 joints): wrist + 5 fingers × 4 joints each
    Hand,
    /// Spider / insect (33 joints): body + 8 legs × 4 segments
    Spider,
    /// Snake / chain (20 joints): sequential chain for rope/tentacle/snake
    Snake,
    /// T-Pose reference (3 joints): minimal root-spine-head for testing
    Minimal,
}

impl SkeletonPreset {
    /// All available presets.
    pub fn all() -> &'static [SkeletonPreset] {
        &[
            SkeletonPreset::Humanoid,
            SkeletonPreset::HumanoidSimple,
            SkeletonPreset::Quadruped,
            SkeletonPreset::Hand,
            SkeletonPreset::Spider,
            SkeletonPreset::Snake,
            SkeletonPreset::Minimal,
        ]
    }

    /// Human-readable name.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Humanoid => "Humanoid (65 joints)",
            Self::HumanoidSimple => "Humanoid Simple (19 joints)",
            Self::Quadruped => "Quadruped (25 joints)",
            Self::Hand => "Hand (21 joints)",
            Self::Spider => "Spider (33 joints)",
            Self::Snake => "Snake (20 joints)",
            Self::Minimal => "Minimal (3 joints)",
        }
    }

    /// Icon for the preset.
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Humanoid | Self::HumanoidSimple => "🧍",
            Self::Quadruped => "🐕",
            Self::Hand => "🤚",
            Self::Spider => "🕷",
            Self::Snake => "🐍",
            Self::Minimal => "🦴",
        }
    }

    /// Short identifier name.
    pub fn short_name(&self) -> &'static str {
        match self {
            Self::Humanoid => "Humanoid",
            Self::HumanoidSimple => "Humanoid_Simple",
            Self::Quadruped => "Quadruped",
            Self::Hand => "Hand",
            Self::Spider => "Spider",
            Self::Snake => "Snake",
            Self::Minimal => "Minimal",
        }
    }

    /// Generate the ImportedModel for this preset.
    pub fn generate(&self) -> ImportedModel {
        match self {
            Self::Humanoid => generate_humanoid(),
            Self::HumanoidSimple => generate_humanoid_simple(),
            Self::Quadruped => generate_quadruped(),
            Self::Hand => generate_hand(),
            Self::Spider => generate_spider(),
            Self::Snake => generate_snake(),
            Self::Minimal => generate_minimal(),
        }
    }
}

// ═══════════════════════════════════════════════════════════
// Helper: build a model from joint definitions
// ═══════════════════════════════════════════════════════════

struct JointDef {
    name: &'static str,
    parent: i32,
    position: Vec3,
}

fn build_model(name: &str, joints: &[JointDef], idle_anim: bool) -> ImportedModel {
    let joint_names: Vec<String> = joints.iter().map(|j| j.name.to_string()).collect();
    let parent_indices: Vec<i32> = joints.iter().map(|j| j.parent).collect();

    // Generate rest-pose frames as Mat4
    let rest_transforms: Vec<Mat4> = joints.iter().map(|j| {
        Mat4::from_translation(j.position)
    }).collect();

    // Optionally generate a subtle idle breathing animation (8 frames)
    let animation = if idle_anim && joints.len() > 2 {
        let num_frames = 60;
        let mut frames = Vec::new();
        for f in 0..num_frames {
            let phase = f as f32 / num_frames as f32 * std::f32::consts::TAU;
            let breath = phase.sin() * 0.01; // subtle Y offset for breathing

            let mut frame_transforms = rest_transforms.clone();
            // Apply subtle Y movement to spine-like joints (indices 1..3)
            for i in 1..joints.len().min(4) {
                let pos = joints[i].position + Vec3::new(0.0, breath * (i as f32), 0.0);
                frame_transforms[i] = Mat4::from_translation(pos);
            }
            frames.push(frame_transforms);
        }
        Some(AnimationData {
            frames,
            framerate: 30.0,
        })
    } else {
        None
    };

    ImportedModel {
        name: name.to_string(),
        meshes: vec![],
        skin: None,
        joint_names,
        parent_indices,
        animation_frames: animation,
    }
}

// ═══════════════════════════════════════════════════════════
// Full Humanoid — 65 joints (standard motion capture layout)
// ═══════════════════════════════════════════════════════════

fn generate_humanoid() -> ImportedModel {
    let joints = vec![
        // 0: Hips (root)
        JointDef { name: "Hips", parent: -1, position: Vec3::new(0.0, 1.0, 0.0) },
        // Spine chain: 1-3
        JointDef { name: "Spine", parent: 0, position: Vec3::new(0.0, 1.1, 0.0) },
        JointDef { name: "Spine1", parent: 1, position: Vec3::new(0.0, 1.25, 0.0) },
        JointDef { name: "Spine2", parent: 2, position: Vec3::new(0.0, 1.4, 0.0) },
        // Neck/Head: 4-5
        JointDef { name: "Neck", parent: 3, position: Vec3::new(0.0, 1.55, 0.0) },
        JointDef { name: "Head", parent: 4, position: Vec3::new(0.0, 1.65, 0.0) },
        JointDef { name: "HeadTop", parent: 5, position: Vec3::new(0.0, 1.8, 0.0) },
        // Left arm: 7-11
        JointDef { name: "LeftShoulder", parent: 3, position: Vec3::new(0.08, 1.5, 0.0) },
        JointDef { name: "LeftArm", parent: 7, position: Vec3::new(0.2, 1.5, 0.0) },
        JointDef { name: "LeftForeArm", parent: 8, position: Vec3::new(0.45, 1.5, 0.0) },
        JointDef { name: "LeftHand", parent: 9, position: Vec3::new(0.7, 1.5, 0.0) },
        // Left fingers: 11-25
        JointDef { name: "LeftThumb1", parent: 10, position: Vec3::new(0.73, 1.49, 0.02) },
        JointDef { name: "LeftThumb2", parent: 11, position: Vec3::new(0.76, 1.48, 0.03) },
        JointDef { name: "LeftThumb3", parent: 12, position: Vec3::new(0.79, 1.47, 0.04) },
        JointDef { name: "LeftIndex1", parent: 10, position: Vec3::new(0.76, 1.5, 0.015) },
        JointDef { name: "LeftIndex2", parent: 14, position: Vec3::new(0.80, 1.5, 0.015) },
        JointDef { name: "LeftIndex3", parent: 15, position: Vec3::new(0.83, 1.5, 0.015) },
        JointDef { name: "LeftMiddle1", parent: 10, position: Vec3::new(0.76, 1.5, 0.0) },
        JointDef { name: "LeftMiddle2", parent: 17, position: Vec3::new(0.80, 1.5, 0.0) },
        JointDef { name: "LeftMiddle3", parent: 18, position: Vec3::new(0.83, 1.5, 0.0) },
        JointDef { name: "LeftRing1", parent: 10, position: Vec3::new(0.76, 1.5, -0.015) },
        JointDef { name: "LeftRing2", parent: 20, position: Vec3::new(0.80, 1.5, -0.015) },
        JointDef { name: "LeftRing3", parent: 21, position: Vec3::new(0.83, 1.5, -0.015) },
        JointDef { name: "LeftPinky1", parent: 10, position: Vec3::new(0.75, 1.5, -0.03) },
        JointDef { name: "LeftPinky2", parent: 23, position: Vec3::new(0.78, 1.5, -0.03) },
        JointDef { name: "LeftPinky3", parent: 24, position: Vec3::new(0.81, 1.5, -0.03) },
        // Right arm: 26-30
        JointDef { name: "RightShoulder", parent: 3, position: Vec3::new(-0.08, 1.5, 0.0) },
        JointDef { name: "RightArm", parent: 26, position: Vec3::new(-0.2, 1.5, 0.0) },
        JointDef { name: "RightForeArm", parent: 27, position: Vec3::new(-0.45, 1.5, 0.0) },
        JointDef { name: "RightHand", parent: 28, position: Vec3::new(-0.7, 1.5, 0.0) },
        // Right fingers: 30-44
        JointDef { name: "RightThumb1", parent: 29, position: Vec3::new(-0.73, 1.49, 0.02) },
        JointDef { name: "RightThumb2", parent: 30, position: Vec3::new(-0.76, 1.48, 0.03) },
        JointDef { name: "RightThumb3", parent: 31, position: Vec3::new(-0.79, 1.47, 0.04) },
        JointDef { name: "RightIndex1", parent: 29, position: Vec3::new(-0.76, 1.5, 0.015) },
        JointDef { name: "RightIndex2", parent: 33, position: Vec3::new(-0.80, 1.5, 0.015) },
        JointDef { name: "RightIndex3", parent: 34, position: Vec3::new(-0.83, 1.5, 0.015) },
        JointDef { name: "RightMiddle1", parent: 29, position: Vec3::new(-0.76, 1.5, 0.0) },
        JointDef { name: "RightMiddle2", parent: 36, position: Vec3::new(-0.80, 1.5, 0.0) },
        JointDef { name: "RightMiddle3", parent: 37, position: Vec3::new(-0.83, 1.5, 0.0) },
        JointDef { name: "RightRing1", parent: 29, position: Vec3::new(-0.76, 1.5, -0.015) },
        JointDef { name: "RightRing2", parent: 39, position: Vec3::new(-0.80, 1.5, -0.015) },
        JointDef { name: "RightRing3", parent: 40, position: Vec3::new(-0.83, 1.5, -0.015) },
        JointDef { name: "RightPinky1", parent: 29, position: Vec3::new(-0.75, 1.5, -0.03) },
        JointDef { name: "RightPinky2", parent: 42, position: Vec3::new(-0.78, 1.5, -0.03) },
        JointDef { name: "RightPinky3", parent: 43, position: Vec3::new(-0.81, 1.5, -0.03) },
        // Left leg: 45-49
        JointDef { name: "LeftUpLeg", parent: 0, position: Vec3::new(0.1, 0.95, 0.0) },
        JointDef { name: "LeftLeg", parent: 45, position: Vec3::new(0.1, 0.52, 0.0) },
        JointDef { name: "LeftFoot", parent: 46, position: Vec3::new(0.1, 0.08, 0.0) },
        JointDef { name: "LeftToeBase", parent: 47, position: Vec3::new(0.1, 0.02, 0.1) },
        JointDef { name: "LeftToeEnd", parent: 48, position: Vec3::new(0.1, 0.0, 0.18) },
        // Right leg: 50-54
        JointDef { name: "RightUpLeg", parent: 0, position: Vec3::new(-0.1, 0.95, 0.0) },
        JointDef { name: "RightLeg", parent: 50, position: Vec3::new(-0.1, 0.52, 0.0) },
        JointDef { name: "RightFoot", parent: 51, position: Vec3::new(-0.1, 0.08, 0.0) },
        JointDef { name: "RightToeBase", parent: 52, position: Vec3::new(-0.1, 0.02, 0.1) },
        JointDef { name: "RightToeEnd", parent: 53, position: Vec3::new(-0.1, 0.0, 0.18) },
        // Eyes: 55-56
        JointDef { name: "LeftEye", parent: 5, position: Vec3::new(0.035, 1.72, 0.06) },
        JointDef { name: "RightEye", parent: 5, position: Vec3::new(-0.035, 1.72, 0.06) },
        // Jaw: 57
        JointDef { name: "Jaw", parent: 5, position: Vec3::new(0.0, 1.62, 0.04) },
        // Extra spine/pelvis detail: 58-64
        JointDef { name: "LeftCollar", parent: 3, position: Vec3::new(0.05, 1.48, 0.0) },
        JointDef { name: "RightCollar", parent: 3, position: Vec3::new(-0.05, 1.48, 0.0) },
        JointDef { name: "LeftHip", parent: 0, position: Vec3::new(0.08, 0.98, 0.0) },
        JointDef { name: "RightHip", parent: 0, position: Vec3::new(-0.08, 0.98, 0.0) },
        JointDef { name: "LeftButtock", parent: 0, position: Vec3::new(0.08, 0.93, -0.05) },
        JointDef { name: "RightButtock", parent: 0, position: Vec3::new(-0.08, 0.93, -0.05) },
        JointDef { name: "Tail", parent: 0, position: Vec3::new(0.0, 0.95, -0.08) },
    ];

    build_model("Humanoid", &joints, true)
}

// ═══════════════════════════════════════════════════════════
// Simple Humanoid — 19 joints (no fingers, basic rig)
// ═══════════════════════════════════════════════════════════

fn generate_humanoid_simple() -> ImportedModel {
    let joints = vec![
        JointDef { name: "Hips", parent: -1, position: Vec3::new(0.0, 1.0, 0.0) },
        JointDef { name: "Spine", parent: 0, position: Vec3::new(0.0, 1.15, 0.0) },
        JointDef { name: "Spine1", parent: 1, position: Vec3::new(0.0, 1.3, 0.0) },
        JointDef { name: "Neck", parent: 2, position: Vec3::new(0.0, 1.5, 0.0) },
        JointDef { name: "Head", parent: 3, position: Vec3::new(0.0, 1.6, 0.0) },
        JointDef { name: "HeadTop", parent: 4, position: Vec3::new(0.0, 1.78, 0.0) },
        // Left arm
        JointDef { name: "LeftShoulder", parent: 2, position: Vec3::new(0.12, 1.48, 0.0) },
        JointDef { name: "LeftArm", parent: 6, position: Vec3::new(0.28, 1.48, 0.0) },
        JointDef { name: "LeftForeArm", parent: 7, position: Vec3::new(0.52, 1.48, 0.0) },
        JointDef { name: "LeftHand", parent: 8, position: Vec3::new(0.72, 1.48, 0.0) },
        // Right arm
        JointDef { name: "RightShoulder", parent: 2, position: Vec3::new(-0.12, 1.48, 0.0) },
        JointDef { name: "RightArm", parent: 10, position: Vec3::new(-0.28, 1.48, 0.0) },
        JointDef { name: "RightForeArm", parent: 11, position: Vec3::new(-0.52, 1.48, 0.0) },
        JointDef { name: "RightHand", parent: 12, position: Vec3::new(-0.72, 1.48, 0.0) },
        // Left leg
        JointDef { name: "LeftUpLeg", parent: 0, position: Vec3::new(0.1, 0.95, 0.0) },
        JointDef { name: "LeftLeg", parent: 14, position: Vec3::new(0.1, 0.5, 0.0) },
        JointDef { name: "LeftFoot", parent: 15, position: Vec3::new(0.1, 0.06, 0.05) },
        // Right leg
        JointDef { name: "RightUpLeg", parent: 0, position: Vec3::new(-0.1, 0.95, 0.0) },
        JointDef { name: "RightLeg", parent: 17, position: Vec3::new(-0.1, 0.5, 0.0) },
        JointDef { name: "RightFoot", parent: 18, position: Vec3::new(-0.1, 0.06, 0.05) },
    ];

    build_model("Humanoid_Simple", &joints, true)
}

// ═══════════════════════════════════════════════════════════
// Quadruped — 25 joints (dog/horse style)
// ═══════════════════════════════════════════════════════════

fn generate_quadruped() -> ImportedModel {
    let joints = vec![
        JointDef { name: "Root", parent: -1, position: Vec3::new(0.0, 0.7, 0.0) },
        JointDef { name: "Spine", parent: 0, position: Vec3::new(0.0, 0.72, 0.15) },
        JointDef { name: "Spine1", parent: 1, position: Vec3::new(0.0, 0.74, 0.30) },
        JointDef { name: "Chest", parent: 2, position: Vec3::new(0.0, 0.76, 0.45) },
        JointDef { name: "Neck", parent: 3, position: Vec3::new(0.0, 0.85, 0.55) },
        JointDef { name: "Head", parent: 4, position: Vec3::new(0.0, 0.92, 0.68) },
        JointDef { name: "Jaw", parent: 5, position: Vec3::new(0.0, 0.88, 0.78) },
        // Front left leg
        JointDef { name: "FrontLeftShoulder", parent: 3, position: Vec3::new(0.12, 0.65, 0.45) },
        JointDef { name: "FrontLeftElbow", parent: 7, position: Vec3::new(0.12, 0.38, 0.45) },
        JointDef { name: "FrontLeftWrist", parent: 8, position: Vec3::new(0.12, 0.10, 0.45) },
        JointDef { name: "FrontLeftPaw", parent: 9, position: Vec3::new(0.12, 0.0, 0.48) },
        // Front right leg
        JointDef { name: "FrontRightShoulder", parent: 3, position: Vec3::new(-0.12, 0.65, 0.45) },
        JointDef { name: "FrontRightElbow", parent: 11, position: Vec3::new(-0.12, 0.38, 0.45) },
        JointDef { name: "FrontRightWrist", parent: 12, position: Vec3::new(-0.12, 0.10, 0.45) },
        JointDef { name: "FrontRightPaw", parent: 13, position: Vec3::new(-0.12, 0.0, 0.48) },
        // Rear left leg
        JointDef { name: "RearLeftHip", parent: 0, position: Vec3::new(0.12, 0.62, -0.1) },
        JointDef { name: "RearLeftKnee", parent: 15, position: Vec3::new(0.12, 0.35, -0.1) },
        JointDef { name: "RearLeftAnkle", parent: 16, position: Vec3::new(0.12, 0.10, -0.08) },
        JointDef { name: "RearLeftPaw", parent: 17, position: Vec3::new(0.12, 0.0, -0.05) },
        // Rear right leg
        JointDef { name: "RearRightHip", parent: 0, position: Vec3::new(-0.12, 0.62, -0.1) },
        JointDef { name: "RearRightKnee", parent: 19, position: Vec3::new(-0.12, 0.35, -0.1) },
        JointDef { name: "RearRightAnkle", parent: 20, position: Vec3::new(-0.12, 0.10, -0.08) },
        JointDef { name: "RearRightPaw", parent: 21, position: Vec3::new(-0.12, 0.0, -0.05) },
        // Tail
        JointDef { name: "Tail1", parent: 0, position: Vec3::new(0.0, 0.72, -0.2) },
        JointDef { name: "Tail2", parent: 23, position: Vec3::new(0.0, 0.78, -0.35) },
    ];

    build_model("Quadruped", &joints, true)
}

// ═══════════════════════════════════════════════════════════
// Hand — 21 joints (wrist + 5 fingers × 4 joints)
// ═══════════════════════════════════════════════════════════

fn generate_hand() -> ImportedModel {
    let joints = vec![
        JointDef { name: "Wrist", parent: -1, position: Vec3::new(0.0, 0.0, 0.0) },
        // Thumb
        JointDef { name: "Thumb_CMC", parent: 0, position: Vec3::new(0.03, 0.01, 0.025) },
        JointDef { name: "Thumb_MCP", parent: 1, position: Vec3::new(0.06, 0.0, 0.045) },
        JointDef { name: "Thumb_IP", parent: 2, position: Vec3::new(0.08, -0.005, 0.06) },
        JointDef { name: "Thumb_Tip", parent: 3, position: Vec3::new(0.095, -0.01, 0.075) },
        // Index
        JointDef { name: "Index_MCP", parent: 0, position: Vec3::new(0.02, 0.0, 0.08) },
        JointDef { name: "Index_PIP", parent: 5, position: Vec3::new(0.02, -0.005, 0.12) },
        JointDef { name: "Index_DIP", parent: 6, position: Vec3::new(0.02, -0.008, 0.15) },
        JointDef { name: "Index_Tip", parent: 7, position: Vec3::new(0.02, -0.01, 0.17) },
        // Middle
        JointDef { name: "Middle_MCP", parent: 0, position: Vec3::new(0.0, 0.0, 0.08) },
        JointDef { name: "Middle_PIP", parent: 9, position: Vec3::new(0.0, -0.005, 0.125) },
        JointDef { name: "Middle_DIP", parent: 10, position: Vec3::new(0.0, -0.008, 0.155) },
        JointDef { name: "Middle_Tip", parent: 11, position: Vec3::new(0.0, -0.01, 0.175) },
        // Ring
        JointDef { name: "Ring_MCP", parent: 0, position: Vec3::new(-0.02, 0.0, 0.078) },
        JointDef { name: "Ring_PIP", parent: 13, position: Vec3::new(-0.02, -0.005, 0.115) },
        JointDef { name: "Ring_DIP", parent: 14, position: Vec3::new(-0.02, -0.008, 0.145) },
        JointDef { name: "Ring_Tip", parent: 15, position: Vec3::new(-0.02, -0.01, 0.165) },
        // Pinky
        JointDef { name: "Pinky_MCP", parent: 0, position: Vec3::new(-0.04, 0.0, 0.072) },
        JointDef { name: "Pinky_PIP", parent: 17, position: Vec3::new(-0.04, -0.005, 0.1) },
        JointDef { name: "Pinky_DIP", parent: 18, position: Vec3::new(-0.04, -0.008, 0.125) },
        JointDef { name: "Pinky_Tip", parent: 19, position: Vec3::new(-0.04, -0.01, 0.14) },
    ];

    build_model("Hand", &joints, false)
}

// ═══════════════════════════════════════════════════════════
// Spider — 33 joints (body segments + 8 legs × 4 segments)
// ═══════════════════════════════════════════════════════════

fn generate_spider() -> ImportedModel {
    let mut joints = Vec::new();

    // Body: root
    joints.push(JointDef { name: "Body", parent: -1, position: Vec3::new(0.0, 0.15, 0.0) });

    // 8 legs, evenly distributed
    let leg_angles = [
        (0.35_f32, 0.10_f32),   // front-left
        (-0.35, 0.10),          // front-right
        (0.25, 0.04),           // mid-front-left
        (-0.25, 0.04),          // mid-front-right
        (0.25, -0.04),          // mid-rear-left
        (-0.25, -0.04),         // mid-rear-right
        (0.35, -0.10),          // rear-left
        (-0.35, -0.10),         // rear-right
    ];

    let leg_names = [
        "FrontLeft", "FrontRight",
        "MidFrontLeft", "MidFrontRight",
        "MidRearLeft", "MidRearRight",
        "RearLeft", "RearRight",
    ];

    for (i, ((dx, dz), name)) in leg_angles.iter().zip(leg_names.iter()).enumerate() {
        let base_idx = 1 + i * 4;
        let coxa_x = *dx;
        let coxa_z = *dz;
        let dir_x = if *dx > 0.0 { 1.0 } else { -1.0 };

        joints.push(JointDef { name: leak_string(&format!("{}_Coxa", name)), parent: 0, position: Vec3::new(coxa_x, 0.14, coxa_z) });
        joints.push(JointDef { name: leak_string(&format!("{}_Femur", name)), parent: base_idx as i32, position: Vec3::new(coxa_x + dir_x * 0.15, 0.12, coxa_z) });
        joints.push(JointDef { name: leak_string(&format!("{}_Tibia", name)), parent: (base_idx + 1) as i32, position: Vec3::new(coxa_x + dir_x * 0.28, 0.06, coxa_z) });
        joints.push(JointDef { name: leak_string(&format!("{}_Tarsus", name)), parent: (base_idx + 2) as i32, position: Vec3::new(coxa_x + dir_x * 0.38, 0.0, coxa_z) });
    }

    build_model("Spider", &joints, true)
}

// ═══════════════════════════════════════════════════════════
// Snake / Chain — 20 sequential joints
// ═══════════════════════════════════════════════════════════

fn generate_snake() -> ImportedModel {
    let num = 20;
    let mut joints = Vec::new();

    for i in 0..num {
        let parent = if i == 0 { -1 } else { (i - 1) as i32 };
        let z = i as f32 * 0.08;
        let y = (i as f32 * 0.4).sin() * 0.02 + 0.05;

        joints.push(JointDef {
            name: leak_string(&format!("Segment_{:02}", i)),
            parent,
            position: Vec3::new(0.0, y, z),
        });
    }

    build_model("Snake", &joints, true)
}

// ═══════════════════════════════════════════════════════════
// Minimal — 3 joints (root-spine-head)
// ═══════════════════════════════════════════════════════════

fn generate_minimal() -> ImportedModel {
    let joints = vec![
        JointDef { name: "Root", parent: -1, position: Vec3::new(0.0, 0.0, 0.0) },
        JointDef { name: "Spine", parent: 0, position: Vec3::new(0.0, 0.5, 0.0) },
        JointDef { name: "Head", parent: 1, position: Vec3::new(0.0, 1.0, 0.0) },
    ];

    build_model("Minimal", &joints, false)
}

// ═══════════════════════════════════════════════════════════
// Utility: leak a String to get &'static str (tiny, bounded)
// ═══════════════════════════════════════════════════════════

fn leak_string(s: &str) -> &'static str {
    Box::leak(s.to_string().into_boxed_str())
}
