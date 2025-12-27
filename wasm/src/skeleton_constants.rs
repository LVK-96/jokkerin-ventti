//! Shared skeleton constants used by both build.rs and runtime code.
//!
//! This module is included by both the build script and the main skeleton module
//! to ensure consistency between compile-time validation and runtime behavior.

// Some constants are only used by build.rs for validation
#![allow(dead_code)]

use glam::Vec3;

/// Default skeleton pose - the standard T-pose used as reference for bone lengths.
/// All keyframe animations should maintain these bone lengths (±tolerance).
pub const DEFAULT_HIPS: Vec3 = Vec3::new(0.0, 0.5, 0.0);
pub const DEFAULT_NECK: Vec3 = Vec3::new(0.0, 1.0, 0.0);
pub const DEFAULT_HEAD: Vec3 = Vec3::new(0.0, 1.15, 0.0);
pub const DEFAULT_LEFT_SHOULDER: Vec3 = Vec3::new(-0.02, 1.0, 0.0);
pub const DEFAULT_LEFT_ELBOW: Vec3 = Vec3::new(-0.18, 0.88, 0.0);
pub const DEFAULT_LEFT_HAND: Vec3 = Vec3::new(-0.35, 0.75, 0.0);
pub const DEFAULT_RIGHT_SHOULDER: Vec3 = Vec3::new(0.02, 1.0, 0.0);
pub const DEFAULT_RIGHT_ELBOW: Vec3 = Vec3::new(0.18, 0.88, 0.0);
pub const DEFAULT_RIGHT_HAND: Vec3 = Vec3::new(0.35, 0.75, 0.0);
pub const DEFAULT_LEFT_HIP: Vec3 = Vec3::new(-0.02, 0.45, 0.0);
pub const DEFAULT_LEFT_KNEE: Vec3 = Vec3::new(-0.15, 0.30, 0.0);
pub const DEFAULT_LEFT_FOOT: Vec3 = Vec3::new(-0.15, 0.0, 0.0);
pub const DEFAULT_RIGHT_HIP: Vec3 = Vec3::new(0.02, 0.45, 0.0);
pub const DEFAULT_RIGHT_KNEE: Vec3 = Vec3::new(0.15, 0.30, 0.0);
pub const DEFAULT_RIGHT_FOOT: Vec3 = Vec3::new(0.15, 0.0, 0.0);

/// Expected bone lengths derived from the default skeleton pose (in meters).
/// These are the target lengths that animation keyframes should match.
pub struct BoneLengths {
    pub spine: f32,     // hips → neck (single spine bone now)
    pub head_neck: f32, // neck → head
    pub clavicle: f32,  // neck → shoulder
    pub upper_arm: f32, // shoulder → elbow
    pub forearm: f32,   // elbow → hand
    pub pelvis: f32,    // hips → hip joint
    pub thigh: f32,     // hip → knee
    pub shin: f32,      // knee → foot
}

impl BoneLengths {
    /// Calculate expected bone lengths from the default skeleton pose constants.
    pub fn from_default() -> Self {
        Self {
            spine: DEFAULT_HIPS.distance(DEFAULT_NECK),
            head_neck: DEFAULT_NECK.distance(DEFAULT_HEAD),
            clavicle: DEFAULT_NECK.distance(DEFAULT_LEFT_SHOULDER),
            upper_arm: DEFAULT_LEFT_SHOULDER.distance(DEFAULT_LEFT_ELBOW),
            forearm: DEFAULT_LEFT_ELBOW.distance(DEFAULT_LEFT_HAND),
            pelvis: DEFAULT_HIPS.distance(DEFAULT_LEFT_HIP),
            thigh: DEFAULT_LEFT_HIP.distance(DEFAULT_LEFT_KNEE),
            shin: DEFAULT_LEFT_KNEE.distance(DEFAULT_LEFT_FOOT),
        }
    }
}
