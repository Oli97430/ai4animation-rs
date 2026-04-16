//! Skeleton definition presets — named bone mappings for known character rigs.
//!
//! Mirrors Python Demos/_ASSETS_/{Cranberry,Geno,Quadruped}/Definitions.py.
//! Each preset provides bone name constants, full-body bone lists,
//! and three-point tracking names (head + hands).

/// Skeleton definition for a known character rig.
pub struct SkeletonDef {
    pub name: &'static str,
    pub hip: &'static str,
    pub neck: &'static str,
    pub head: &'static str,
    pub spine: &'static [&'static str],
    pub left_arm: ArmDef,
    pub right_arm: ArmDef,
    pub left_leg: LegDef,
    pub right_leg: LegDef,
    pub full_body: &'static [&'static str],
    /// Three-point tracking: [head, left_hand, right_hand].
    pub three_point: Option<[&'static str; 3]>,
}

pub struct ArmDef {
    pub shoulder: &'static str,
    pub upper: &'static str,
    pub forearm: &'static str,
    pub hand: &'static str,
}

pub struct LegDef {
    pub hip: &'static str,
    pub knee: &'static str,
    pub ankle: &'static str,
    pub ball: &'static str,
}

// ── Cranberry (Meta internal rig) ──────────────────────────────────

pub static CRANBERRY_FULL_BODY: &[&str] = &[
    "b_root",
    "b_l_upleg", "b_l_leg", "b_l_talocrural", "b_l_ball",
    "b_r_upleg", "b_r_leg", "b_r_talocrural", "b_r_ball",
    "b_spine0", "b_spine1", "b_spine2", "b_spine3",
    "b_neck0", "b_head",
    "b_l_shoulder", "p_l_scap", "b_l_arm", "b_l_forearm", "b_l_wrist_twist", "b_l_wrist",
    "b_r_shoulder", "p_r_scap", "b_r_arm", "b_r_forearm", "b_r_wrist_twist", "b_r_wrist",
];

pub static CRANBERRY: SkeletonDef = SkeletonDef {
    name: "Cranberry",
    hip: "b_root",
    neck: "b_neck0",
    head: "b_head",
    spine: &["b_spine0", "b_spine1", "b_spine2", "b_spine3"],
    left_arm: ArmDef {
        shoulder: "b_l_shoulder",
        upper: "b_l_arm",
        forearm: "b_l_forearm",
        hand: "b_l_wrist",
    },
    right_arm: ArmDef {
        shoulder: "b_r_shoulder",
        upper: "b_r_arm",
        forearm: "b_r_forearm",
        hand: "b_r_wrist",
    },
    left_leg: LegDef {
        hip: "b_l_upleg",
        knee: "b_l_leg",
        ankle: "b_l_talocrural",
        ball: "b_l_ball",
    },
    right_leg: LegDef {
        hip: "b_r_upleg",
        knee: "b_r_leg",
        ankle: "b_r_talocrural",
        ball: "b_r_ball",
    },
    full_body: CRANBERRY_FULL_BODY,
    three_point: Some(["b_head", "b_l_wrist", "b_r_wrist"]),
};

// ── Geno (standard Mixamo/BVH rig) ────────────────────────────────

pub static GENO_FULL_BODY: &[&str] = &[
    "Hips",
    "LeftUpLeg", "LeftLeg", "LeftFoot", "LeftToeBase",
    "RightUpLeg", "RightLeg", "RightFoot", "RightToeBase",
    "Spine", "Spine1", "Spine2", "Spine3",
    "Neck", "Head",
    "LeftShoulder", "LeftArm", "LeftForeArm", "LeftHand",
    "RightShoulder", "RightArm", "RightForeArm", "RightHand",
];

pub static GENO: SkeletonDef = SkeletonDef {
    name: "Geno",
    hip: "Hips",
    neck: "Neck",
    head: "Head",
    spine: &["Spine", "Spine1", "Spine2", "Spine3"],
    left_arm: ArmDef {
        shoulder: "LeftShoulder",
        upper: "LeftArm",
        forearm: "LeftForeArm",
        hand: "LeftHand",
    },
    right_arm: ArmDef {
        shoulder: "RightShoulder",
        upper: "RightArm",
        forearm: "RightForeArm",
        hand: "RightHand",
    },
    left_leg: LegDef {
        hip: "LeftUpLeg",
        knee: "LeftLeg",
        ankle: "LeftFoot",
        ball: "LeftToeBase",
    },
    right_leg: LegDef {
        hip: "RightUpLeg",
        knee: "RightLeg",
        ankle: "RightFoot",
        ball: "RightToeBase",
    },
    full_body: GENO_FULL_BODY,
    three_point: Some(["Head", "LeftHand", "RightHand"]),
};

// ── Quadruped ──────────────────────────────────────────────────────

pub static QUADRUPED_FULL_BODY: &[&str] = &[
    "Hips",
    "Spine", "Spine1",
    "Neck", "Head", "HeadSite",
    "LeftShoulder", "LeftArm", "LeftForeArm", "LeftHand", "LeftHandSite",
    "RightShoulder", "RightArm", "RightForeArm", "RightHand", "RightHandSite",
    "LeftUpLeg", "LeftLeg", "LeftFoot", "LeftFootSite",
    "RightUpLeg", "RightLeg", "RightFoot", "RightFootSite",
    "Tail", "Tail1", "Tail1Site",
];

pub static QUADRUPED: SkeletonDef = SkeletonDef {
    name: "Quadruped",
    hip: "Hips",
    neck: "Neck",
    head: "Head",
    spine: &["Spine", "Spine1"],
    left_arm: ArmDef {
        shoulder: "LeftShoulder",
        upper: "LeftArm",
        forearm: "LeftForeArm",
        hand: "LeftHand",
    },
    right_arm: ArmDef {
        shoulder: "RightShoulder",
        upper: "RightArm",
        forearm: "RightForeArm",
        hand: "RightHand",
    },
    left_leg: LegDef {
        hip: "LeftUpLeg",
        knee: "LeftLeg",
        ankle: "LeftFoot",
        ball: "LeftFoot", // Quadruped has no separate ball/toe
    },
    right_leg: LegDef {
        hip: "RightUpLeg",
        knee: "RightLeg",
        ankle: "RightFoot",
        ball: "RightFoot",
    },
    full_body: QUADRUPED_FULL_BODY,
    three_point: None, // No three-point tracking for quadrupeds
};

/// Find a skeleton definition by name.
pub fn find_def(name: &str) -> Option<&'static SkeletonDef> {
    match name.to_lowercase().as_str() {
        "cranberry" => Some(&CRANBERRY),
        "geno" | "mixamo" => Some(&GENO),
        "quadruped" => Some(&QUADRUPED),
        _ => None,
    }
}

/// All available skeleton definitions.
pub fn all_defs() -> Vec<&'static SkeletonDef> {
    vec![&CRANBERRY, &GENO, &QUADRUPED]
}
