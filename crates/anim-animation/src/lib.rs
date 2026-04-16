//! Animation system: Motion, Hierarchy, Modules, Actor.

pub mod hierarchy;
pub mod motion;
pub mod contact;
pub mod trajectory;
pub mod guidance;
pub mod tracking;
pub mod root_motion;
pub mod retarget;
pub mod time_series;
pub mod motion_module;
pub mod dataset;
pub mod phase;
pub mod actor;
pub mod mesh_renderer;
pub mod running_stats;
pub mod skeleton_defs;
pub mod onnx_inference;
pub mod locomotion;
pub mod blend;
pub mod motion_matching;
pub mod state_machine;
pub mod blend_tree;
pub mod ragdoll;
pub mod deep_phase;
pub mod anim_recorder;
pub mod cloth;
pub mod keyframe;
pub mod shape_keys;
pub mod camera_anim;
pub mod particles;

pub use hierarchy::Hierarchy;
pub use motion::Motion;
pub use contact::ContactModule;
pub use trajectory::{Trajectory, TrajectoryConfig};
pub use guidance::GuidanceModule;
pub use tracking::TrackingModule;
pub use root_motion::{RootMotion, RootConfig, Topology};
pub use retarget::{RetargetMap, build_retarget};
pub use time_series::TimeSeries;
pub use motion_module::{MotionFeatures, compute_features};
pub use dataset::Dataset;
pub use phase::{PhaseData, detect_phase};
pub use actor::{Actor, Bone};
pub use mesh_renderer::MeshRenderer;
pub use running_stats::RunningStats;
pub use onnx_inference::{OnnxModel, OnnxCache, ModelMetadata};
pub use locomotion::LocomotionController;
pub use blend::{
    blend_poses, blend_positions, blend_velocities,
    AnimationLayer, AnimationTransition, BlendNode, BlendMode, EasingCurve,
    apply_layers,
};
pub use motion_matching::{
    MotionDatabase, MotionMatchingController, MatchWeights, TrajectoryFeatureConfig,
};
pub use state_machine::{
    StateMachine, TransitionCondition, CompareOp, MotionSource, StateChangeEvent,
};
pub use blend_tree::{
    BlendTree, BlendTreeNode, ClipNode, Blend1DNode, Blend2DNode, LerpNode, BlendResult,
};
pub use ragdoll::{Ragdoll, RagdollConfig, RigidBody};
pub use deep_phase::{
    DeepPhaseManifold, DeepPhaseConfig, PhaseState, ChannelGroup,
    extract_deep_phase, transition_score, find_best_transition,
};
pub use anim_recorder::{AnimRecorder, RecorderConfig, RecordedClip, RecordingState, clip_to_motion_data};
pub use cloth::{ClothSim, ClothConfig, Particle, DistanceConstraint};
pub use keyframe::{KeyframeAnimation, KeyframeLayer, KeyframeTrack, Keyframe, TweenType};
pub use shape_keys::{ShapeKey, ShapeKeySet};
pub use camera_anim::{CameraAnimation, CameraKeyframe, CameraAnimPlayer, CameraState, CameraEasing};
pub use particles::{ParticleEmitter, EmitterConfig, EmissionShape};
