//! Animation math library wrapping glam with batch operations via ndarray.
//! Mirrors the Python ai4animation Math/ modules.

pub mod transform;
pub mod quaternion;
pub mod rotation;
pub mod vector3;
pub mod batch;
pub mod signal;
pub mod utility;

pub use glam::{Mat4, Mat3, Quat, Vec3, Vec4, EulerRot};
pub use transform::Transform;
pub use rotation::Rotation;
pub use quaternion::QuatExt;
pub use vector3::Vec3Ext;
pub use utility::{normalize, ratio, center_of_mass, bounding_box};
