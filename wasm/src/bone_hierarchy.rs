//! Bone hierarchy and rotation-based pose system
//!
//! This module defines the skeleton as a hierarchical tree of bones with
//! fixed lengths. Animation is done via quaternion rotations, which guarantees
//! bone lengths are preserved.
//!
//! ## Key Concepts
//!
//! - **BoneId**: Enum identifying each bone in the skeleton
//! - **BONE_HIERARCHY**: Static definition of parent-child relationships and rest lengths
//! - **RotationPose**: Animation pose using local quaternion rotations
//! - **Lazy FK**: Forward kinematics with dirty flag tracking for efficiency

#![allow(dead_code)] // Module is new, will be integrated incrementally

use glam::{Quat, Vec3};

/// Unique identifier for each bone in the skeleton.
/// Ordered for topological traversal (parents before children).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum BoneId {
    // Root
    Hips = 0,

    // Spine chain
    Spine = 1, // hips -> neck
    Neck = 2,  // For head attachment
    Head = 3,

    // Left arm chain
    LeftShoulder = 4,
    LeftUpperArm = 5,
    LeftForearm = 6,

    // Right arm chain
    RightShoulder = 7,
    RightUpperArm = 8,
    RightForearm = 9,

    // Left leg chain
    LeftThigh = 10,
    LeftShin = 11,

    // Right leg chain
    RightThigh = 12,
    RightShin = 13,
}

impl BoneId {
    /// Total number of bones in the skeleton
    pub const COUNT: usize = 14;

    /// Convert to array index
    #[inline]
    pub const fn index(self) -> usize {
        self as usize
    }

    /// Get all bone IDs in topological order (parents before children)
    pub const ALL: [BoneId; Self::COUNT] = [
        BoneId::Hips,
        BoneId::Spine,
        BoneId::Neck,
        BoneId::Head,
        BoneId::LeftShoulder,
        BoneId::LeftUpperArm,
        BoneId::LeftForearm,
        BoneId::RightShoulder,
        BoneId::RightUpperArm,
        BoneId::RightForearm,
        BoneId::LeftThigh,
        BoneId::LeftShin,
        BoneId::RightThigh,
        BoneId::RightShin,
    ];
}

/// Static bone definition: parent relationship and rest-pose length
#[derive(Debug, Clone, Copy)]
pub struct BoneDef {
    /// Parent bone (None for root)
    pub parent: Option<BoneId>,
    /// Length of this bone in meters (distance to child joint)
    pub length: f32,
    /// Direction vector in parent's local space (unit vector)
    pub direction: Vec3,
}

/// Bone hierarchy definition with rest-pose geometry.
/// Lengths derived from skeleton_constants.rs bind pose.
pub const BONE_HIERARCHY: [BoneDef; BoneId::COUNT] = [
    // Hips - root bone, no parent
    BoneDef {
        parent: None,
        length: 0.0,
        direction: Vec3::Y,
    },
    // Spine - hips to neck (0.5m upward)
    BoneDef {
        parent: Some(BoneId::Hips),
        length: 0.50,
        direction: Vec3::Y,
    },
    // Neck - short segment at top of spine
    BoneDef {
        parent: Some(BoneId::Spine),
        length: 0.0,
        direction: Vec3::Y,
    },
    // Head - neck to head (0.15m upward)
    BoneDef {
        parent: Some(BoneId::Neck),
        length: 0.15,
        direction: Vec3::Y,
    },
    // Left shoulder - neck to shoulder (outward)
    BoneDef {
        parent: Some(BoneId::Neck),
        length: 0.02,
        direction: Vec3::NEG_X,
    },
    // Left upper arm - shoulder to elbow (0.2m)
    BoneDef {
        parent: Some(BoneId::LeftShoulder),
        length: 0.20,
        direction: Vec3::new(-0.8, -0.6, 0.0),
    },
    // Left forearm - elbow to hand (0.214m)
    BoneDef {
        parent: Some(BoneId::LeftUpperArm),
        length: 0.214,
        direction: Vec3::new(-0.8, -0.6, 0.0),
    },
    // Right shoulder - neck to shoulder (outward)
    BoneDef {
        parent: Some(BoneId::Neck),
        length: 0.02,
        direction: Vec3::X,
    },
    // Right upper arm - shoulder to elbow (0.2m)
    BoneDef {
        parent: Some(BoneId::RightShoulder),
        length: 0.20,
        direction: Vec3::new(0.8, -0.6, 0.0),
    },
    // Right forearm - elbow to hand (0.214m)
    BoneDef {
        parent: Some(BoneId::RightUpperArm),
        length: 0.214,
        direction: Vec3::new(0.8, -0.6, 0.0),
    },
    // Left thigh - hip to knee (0.198m, adjusted for hip offset)
    BoneDef {
        parent: Some(BoneId::Hips),
        length: 0.198,
        direction: Vec3::new(-0.65, -0.9, 0.0),
    },
    // Left shin - knee to foot (0.3m)
    BoneDef {
        parent: Some(BoneId::LeftThigh),
        length: 0.30,
        direction: Vec3::NEG_Y,
    },
    // Right thigh - hip to knee (0.198m)
    BoneDef {
        parent: Some(BoneId::Hips),
        length: 0.198,
        direction: Vec3::new(0.65, -0.9, 0.0),
    },
    // Right shin - knee to foot (0.3m)
    BoneDef {
        parent: Some(BoneId::RightThigh),
        length: 0.30,
        direction: Vec3::NEG_Y,
    },
];

/// Dirty flags for lazy forward kinematics evaluation.
/// Uses a bitset where bit i corresponds to BoneId with index i.
#[derive(Debug, Clone, Copy, Default)]
pub struct DirtyFlags(u16);

impl DirtyFlags {
    /// Create with all bones marked dirty
    pub fn all_dirty() -> Self {
        Self((1 << BoneId::COUNT) - 1)
    }

    /// Check if a bone is dirty (needs recomputation)
    #[inline]
    pub fn is_dirty(&self, bone: BoneId) -> bool {
        (self.0 & (1 << bone.index())) != 0
    }

    /// Mark a bone and all its children as dirty
    pub fn mark_dirty(&mut self, bone: BoneId) {
        // Mark this bone
        self.0 |= 1 << bone.index();

        // Mark all children (bones that have this as ancestor)
        for child in BoneId::ALL.iter().skip(bone.index() + 1) {
            if Self::is_descendant_of(*child, bone) {
                self.0 |= 1 << child.index();
            }
        }
    }

    /// Clear dirty flag for a bone
    #[inline]
    pub fn clear(&mut self, bone: BoneId) {
        self.0 &= !(1 << bone.index());
    }

    /// Clear all dirty flags
    #[inline]
    pub fn clear_all(&mut self) {
        self.0 = 0;
    }

    /// Check if child is a descendant of ancestor
    fn is_descendant_of(child: BoneId, ancestor: BoneId) -> bool {
        let mut current = child;
        loop {
            if let Some(parent) = BONE_HIERARCHY[current.index()].parent {
                if parent == ancestor {
                    return true;
                }
                current = parent;
            } else {
                return false;
            }
        }
    }
}

/// Rotation-based pose for animation.
///
/// Each bone stores a local rotation (relative to parent).
/// World positions are computed via forward kinematics.
#[derive(Debug, Clone)]
pub struct RotationPose {
    /// Root position in world space
    pub root_position: Vec3,

    /// Local rotation for each bone (relative to parent)
    pub local_rotations: [Quat; BoneId::COUNT],

    /// Cached world transforms (position, rotation)
    /// Lazily computed when needed
    world_positions: [Vec3; BoneId::COUNT],
    world_rotations: [Quat; BoneId::COUNT],

    /// Dirty flags for lazy evaluation
    dirty: DirtyFlags,
}

impl Default for RotationPose {
    fn default() -> Self {
        Self::bind_pose()
    }
}

impl RotationPose {
    /// Create the bind pose (T-pose) with all rotations at identity
    pub fn bind_pose() -> Self {
        let root_position = Vec3::new(0.0, 0.55, 0.0); // Hips position from skeleton_constants

        Self {
            root_position,
            local_rotations: [Quat::IDENTITY; BoneId::COUNT],
            world_positions: [Vec3::ZERO; BoneId::COUNT],
            world_rotations: [Quat::IDENTITY; BoneId::COUNT],
            dirty: DirtyFlags::all_dirty(),
        }
    }

    /// Set the local rotation for a bone (marks it and children dirty)
    pub fn set_rotation(&mut self, bone: BoneId, rotation: Quat) {
        if self.local_rotations[bone.index()] != rotation {
            self.local_rotations[bone.index()] = rotation;
            self.dirty.mark_dirty(bone);
        }
    }

    /// Set root position (marks all bones dirty)
    pub fn set_root_position(&mut self, position: Vec3) {
        if self.root_position != position {
            self.root_position = position;
            self.dirty = DirtyFlags::all_dirty();
        }
    }

    /// Get world position of a bone's end joint (computes FK if needed)
    pub fn get_position(&mut self, bone: BoneId) -> Vec3 {
        self.ensure_computed(bone);
        self.world_positions[bone.index()]
    }

    /// Ensure a bone's world transform is computed
    fn ensure_computed(&mut self, bone: BoneId) {
        if !self.dirty.is_dirty(bone) {
            return;
        }

        // Compute all ancestors first (they're ordered topologically)
        for ancestor in BoneId::ALL.iter().take(bone.index()) {
            if self.dirty.is_dirty(*ancestor) {
                self.compute_bone(*ancestor);
            }
        }

        // Compute this bone
        self.compute_bone(bone);
    }

    /// Compute the world transform for a single bone
    fn compute_bone(&mut self, bone: BoneId) {
        let def = &BONE_HIERARCHY[bone.index()];
        let local_rot = self.local_rotations[bone.index()];

        let (parent_pos, parent_rot) = if let Some(parent) = def.parent {
            (
                self.world_positions[parent.index()],
                self.world_rotations[parent.index()],
            )
        } else {
            // Root bone
            (self.root_position, Quat::IDENTITY)
        };

        // Apply hip offsets for thighs to ensure legs are connected to hips visually
        // This corresponds to the pelvic width
        let parent_pos = if bone == BoneId::LeftThigh {
            parent_pos + parent_rot * Vec3::new(-0.02, -0.05, 0.0)
        } else if bone == BoneId::RightThigh {
            parent_pos + parent_rot * Vec3::new(0.02, -0.05, 0.0)
        } else {
            parent_pos
        };

        // World rotation = parent rotation * local rotation
        let world_rot = parent_rot * local_rot;

        // World position = parent position + rotated bone vector
        let bone_vector = world_rot * (def.direction.normalize() * def.length);
        let world_pos = parent_pos + bone_vector;

        self.world_rotations[bone.index()] = world_rot;
        self.world_positions[bone.index()] = world_pos;
        self.dirty.clear(bone);
    }

    /// Force recomputation of all bones (useful after bulk updates)
    pub fn compute_all(&mut self) {
        for bone in BoneId::ALL {
            self.compute_bone(bone);
        }
        self.dirty.clear_all();
    }

    /// Convert to the old Skeleton format for rendering compatibility
    pub fn to_skeleton(&mut self) -> crate::skeleton::Skeleton {
        self.compute_all();

        use glam::Vec3A;

        // Map rotation pose joints to skeleton positions
        crate::skeleton::Skeleton {
            hips: Vec3A::from(self.root_position),
            neck: Vec3A::from(self.world_positions[BoneId::Spine.index()]),
            head: Vec3A::from(self.world_positions[BoneId::Head.index()]),
            left_shoulder: Vec3A::from(self.world_positions[BoneId::LeftShoulder.index()]),
            left_elbow: Vec3A::from(self.world_positions[BoneId::LeftUpperArm.index()]),
            left_hand: Vec3A::from(self.world_positions[BoneId::LeftForearm.index()]),
            right_shoulder: Vec3A::from(self.world_positions[BoneId::RightShoulder.index()]),
            right_elbow: Vec3A::from(self.world_positions[BoneId::RightUpperArm.index()]),
            right_hand: Vec3A::from(self.world_positions[BoneId::RightForearm.index()]),

            // Calculate actual hip positions based on root rotation + offset
            left_hip: Vec3A::from(
                self.root_position
                    + self.world_rotations[BoneId::Hips.index()] * Vec3::new(-0.02, -0.05, 0.0),
            ),
            left_knee: Vec3A::from(self.world_positions[BoneId::LeftThigh.index()]),
            left_foot: Vec3A::from(self.world_positions[BoneId::LeftShin.index()]),

            right_hip: Vec3A::from(
                self.root_position
                    + self.world_rotations[BoneId::Hips.index()] * Vec3::new(0.02, -0.05, 0.0),
            ),
            right_knee: Vec3A::from(self.world_positions[BoneId::RightThigh.index()]),
            right_foot: Vec3A::from(self.world_positions[BoneId::RightShin.index()]),
        }
    }

    /// Interpolate between two poses using spherical linear interpolation (slerp)
    pub fn lerp(a: &RotationPose, b: &RotationPose, t: f32) -> RotationPose {
        let mut result = RotationPose::bind_pose();

        // Lerp root position
        result.root_position = a.root_position.lerp(b.root_position, t);

        // Slerp all rotations
        for i in 0..BoneId::COUNT {
            result.local_rotations[i] = a.local_rotations[i].slerp(b.local_rotations[i], t);
        }

        // Mark all dirty since we've modified everything
        result.dirty = DirtyFlags::all_dirty();

        result
    }
}

// ============================================================================
// Animation System
// ============================================================================

/// Euler angles in degrees for JSON authoring (more intuitive than quaternions)
#[derive(Debug, Clone, Copy, Default, serde::Deserialize, serde::Serialize)]
pub struct EulerAngles {
    #[serde(default)]
    pub x: f32,
    #[serde(default)]
    pub y: f32,
    #[serde(default)]
    pub z: f32,
}

impl EulerAngles {
    /// Convert to quaternion (XYZ order)
    pub fn to_quat(&self) -> Quat {
        Quat::from_euler(
            glam::EulerRot::XYZ,
            self.x.to_radians(),
            self.y.to_radians(),
            self.z.to_radians(),
        )
    }
    /// Convert from quaternion (XYZ order)
    pub fn from_quat(q: Quat) -> Self {
        let (x, y, z) = q.to_euler(glam::EulerRot::XYZ);
        Self {
            x: x.to_degrees(),
            y: y.to_degrees(),
            z: z.to_degrees(),
        }
    }
}

/// A single keyframe's bone rotations in JSON format
#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct RotationPoseJson {
    /// Root position override (optional)
    #[serde(default)]
    pub root_position: Option<[f32; 3]>,

    /// Root/hips rotation
    #[serde(default)]
    pub hips: Option<EulerAngles>,

    #[serde(default)]
    pub spine: Option<EulerAngles>,

    #[serde(default)]
    pub head: Option<EulerAngles>,

    #[serde(default)]
    pub left_upper_arm: Option<EulerAngles>,

    #[serde(default)]
    pub left_forearm: Option<EulerAngles>,

    #[serde(default)]
    pub right_upper_arm: Option<EulerAngles>,

    #[serde(default)]
    pub right_forearm: Option<EulerAngles>,

    #[serde(default)]
    pub left_thigh: Option<EulerAngles>,

    #[serde(default)]
    pub left_shin: Option<EulerAngles>,

    #[serde(default)]
    pub right_thigh: Option<EulerAngles>,

    #[serde(default)]
    pub right_shin: Option<EulerAngles>,
}

impl RotationPoseJson {
    /// Convert JSON pose to RotationPose
    pub fn to_rotation_pose(&self) -> RotationPose {
        let mut pose = RotationPose::bind_pose();

        // Apply root position if specified
        if let Some([x, y, z]) = self.root_position {
            pose.root_position = Vec3::new(x, y, z);
        }

        // Apply rotations for each bone if specified
        if let Some(euler) = self.hips {
            pose.local_rotations[BoneId::Hips.index()] = euler.to_quat();
        }
        if let Some(euler) = self.spine {
            pose.local_rotations[BoneId::Spine.index()] = euler.to_quat();
        }
        if let Some(euler) = self.head {
            pose.local_rotations[BoneId::Head.index()] = euler.to_quat();
        }
        if let Some(euler) = self.left_upper_arm {
            pose.local_rotations[BoneId::LeftUpperArm.index()] = euler.to_quat();
        }
        if let Some(euler) = self.left_forearm {
            pose.local_rotations[BoneId::LeftForearm.index()] = euler.to_quat();
        }
        if let Some(euler) = self.right_upper_arm {
            pose.local_rotations[BoneId::RightUpperArm.index()] = euler.to_quat();
        }
        if let Some(euler) = self.right_forearm {
            pose.local_rotations[BoneId::RightForearm.index()] = euler.to_quat();
        }
        if let Some(euler) = self.left_thigh {
            pose.local_rotations[BoneId::LeftThigh.index()] = euler.to_quat();
        }
        if let Some(euler) = self.left_shin {
            pose.local_rotations[BoneId::LeftShin.index()] = euler.to_quat();
        }
        if let Some(euler) = self.right_thigh {
            pose.local_rotations[BoneId::RightThigh.index()] = euler.to_quat();
        }
        if let Some(euler) = self.right_shin {
            pose.local_rotations[BoneId::RightShin.index()] = euler.to_quat();
        }

        pose
    }
}

/// A keyframe in a rotation-based animation
#[derive(Debug, Clone)]
pub struct RotationKeyframe {
    pub time: f32,
    pub pose: RotationPose,
}

/// JSON format for keyframe
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct RotationKeyframeJson {
    pub time: f32,
    pub pose: RotationPoseJson,
}

/// Rotation-based animation clip
#[derive(Debug, Clone)]
pub struct RotationAnimationClip {
    pub name: String,
    pub duration: f32,
    pub keyframes: Vec<RotationKeyframe>,
}

/// JSON format for animation clip
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct RotationAnimationClipJson {
    #[serde(rename = "$schema", default, skip_deserializing)]
    pub schema: Option<String>,
    #[serde(default = "default_version")]
    pub version: u32,
    pub name: String,
    pub duration: f32,
    pub keyframes: Vec<RotationKeyframeJson>,
}

fn default_version() -> u32 {
    2
}

impl RotationAnimationClip {
    /// Parse from JSON string
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        let clip_json: RotationAnimationClipJson = serde_json::from_str(json)?;

        let keyframes = clip_json
            .keyframes
            .into_iter()
            .map(|kf| RotationKeyframe {
                time: kf.time,
                pose: kf.pose.to_rotation_pose(),
            })
            .collect();

        Ok(Self {
            name: clip_json.name,
            duration: clip_json.duration,
            keyframes,
        })
    }

    /// Convert to JSON string
    pub fn to_json_string(&self) -> Result<String, serde_json::Error> {
        let keyframes_json: Vec<RotationKeyframeJson> = self
            .keyframes
            .iter()
            .map(|kf| RotationKeyframeJson {
                time: kf.time,
                pose: kf.pose.to_json(),
            })
            .collect();

        let json_struct = RotationAnimationClipJson {
            schema: Some("../../../schemas/animation.v2.schema.json".to_string()),
            version: 2,
            name: self.name.clone(),
            duration: self.duration,
            keyframes: keyframes_json,
        };

        serde_json::to_string_pretty(&json_struct)
    }
}

impl RotationPose {
    /// Convert to JSON representation
    pub fn to_json(&self) -> RotationPoseJson {
        let mut json = RotationPoseJson::default();

        // Only include root position if non-zero
        if self.root_position.length_squared() > 1e-6 {
            json.root_position = Some(self.root_position.to_array());
        }

        let is_identity = |q: Quat| q.angle_between(Quat::IDENTITY) < 1e-4;

        if !is_identity(self.local_rotations[BoneId::Hips.index()]) {
            json.hips = Some(EulerAngles::from_quat(
                self.local_rotations[BoneId::Hips.index()],
            ));
        }
        if !is_identity(self.local_rotations[BoneId::Spine.index()]) {
            json.spine = Some(EulerAngles::from_quat(
                self.local_rotations[BoneId::Spine.index()],
            ));
        }
        if !is_identity(self.local_rotations[BoneId::Head.index()]) {
            json.head = Some(EulerAngles::from_quat(
                self.local_rotations[BoneId::Head.index()],
            ));
        }
        if !is_identity(self.local_rotations[BoneId::LeftUpperArm.index()]) {
            json.left_upper_arm = Some(EulerAngles::from_quat(
                self.local_rotations[BoneId::LeftUpperArm.index()],
            ));
        }
        if !is_identity(self.local_rotations[BoneId::LeftForearm.index()]) {
            json.left_forearm = Some(EulerAngles::from_quat(
                self.local_rotations[BoneId::LeftForearm.index()],
            ));
        }
        if !is_identity(self.local_rotations[BoneId::RightUpperArm.index()]) {
            json.right_upper_arm = Some(EulerAngles::from_quat(
                self.local_rotations[BoneId::RightUpperArm.index()],
            ));
        }
        if !is_identity(self.local_rotations[BoneId::RightForearm.index()]) {
            json.right_forearm = Some(EulerAngles::from_quat(
                self.local_rotations[BoneId::RightForearm.index()],
            ));
        }
        if !is_identity(self.local_rotations[BoneId::LeftThigh.index()]) {
            json.left_thigh = Some(EulerAngles::from_quat(
                self.local_rotations[BoneId::LeftThigh.index()],
            ));
        }
        if !is_identity(self.local_rotations[BoneId::LeftShin.index()]) {
            json.left_shin = Some(EulerAngles::from_quat(
                self.local_rotations[BoneId::LeftShin.index()],
            ));
        }
        if !is_identity(self.local_rotations[BoneId::RightThigh.index()]) {
            json.right_thigh = Some(EulerAngles::from_quat(
                self.local_rotations[BoneId::RightThigh.index()],
            ));
        }
        if !is_identity(self.local_rotations[BoneId::RightShin.index()]) {
            json.right_shin = Some(EulerAngles::from_quat(
                self.local_rotations[BoneId::RightShin.index()],
            ));
        }

        json
    }
}

impl RotationAnimationClip {
    /// Sample the animation at a given time, using slerp interpolation
    pub fn sample(&self, time: f32) -> RotationPose {
        if self.keyframes.is_empty() {
            return RotationPose::bind_pose();
        }

        // Loop time within duration
        let looped_time = time % self.duration;

        // Binary search for keyframe (using partition_point for efficiency)
        let next_idx = self.keyframes.partition_point(|kf| kf.time <= looped_time);

        if next_idx == 0 {
            // Before first keyframe
            return self.keyframes[0].pose.clone();
        }

        if next_idx >= self.keyframes.len() {
            // After last keyframe - interpolate to first for looping
            let prev = &self.keyframes[self.keyframes.len() - 1];
            let next = &self.keyframes[0];
            let segment_duration = self.duration - prev.time + next.time;
            let local_time = looped_time - prev.time;
            let t = if segment_duration > 0.0 {
                local_time / segment_duration
            } else {
                0.0
            };
            return RotationPose::lerp(&prev.pose, &next.pose, t);
        }

        // Between two keyframes
        let prev = &self.keyframes[next_idx - 1];
        let next = &self.keyframes[next_idx];
        let segment_duration = next.time - prev.time;
        let local_time = looped_time - prev.time;

        let t = if segment_duration > 0.0 {
            local_time / segment_duration
        } else {
            0.0
        };

        RotationPose::lerp(&prev.pose, &next.pose, t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bind_pose_positions() {
        let mut pose = RotationPose::bind_pose();
        let skeleton = pose.to_skeleton();

        // Hips should be at root position
        assert!((skeleton.hips.y - 0.55).abs() < 0.01);

        // Head should be above hips
        assert!(skeleton.head.y > skeleton.hips.y);

        // Feet should be near ground
        assert!(skeleton.left_foot.y < 0.1);
        assert!(skeleton.right_foot.y < 0.1);
    }

    #[test]
    fn test_lazy_evaluation() {
        let mut pose = RotationPose::bind_pose();

        // Initially all dirty
        assert!(pose.dirty.is_dirty(BoneId::Head));

        // Access head position - should compute
        let _ = pose.get_position(BoneId::Head);

        // Now computed bones should be clean
        assert!(!pose.dirty.is_dirty(BoneId::Hips));
        assert!(!pose.dirty.is_dirty(BoneId::Spine));
        assert!(!pose.dirty.is_dirty(BoneId::Head));
    }

    #[test]
    fn test_dirty_propagation() {
        let mut pose = RotationPose::bind_pose();
        pose.compute_all();

        // All clean now
        assert!(!pose.dirty.is_dirty(BoneId::Head));

        // Rotate spine - should dirty head (child) but not legs
        pose.set_rotation(BoneId::Spine, Quat::from_rotation_x(0.5));

        assert!(pose.dirty.is_dirty(BoneId::Spine));
        assert!(pose.dirty.is_dirty(BoneId::Head)); // Child of spine
        assert!(!pose.dirty.is_dirty(BoneId::LeftThigh)); // Not a child
    }

    #[test]
    fn test_euler_to_quat() {
        let euler = EulerAngles {
            x: 90.0,
            y: 0.0,
            z: 0.0,
        };
        let quat = euler.to_quat();

        // 90 degree rotation around X: Y axis rotates toward -Z (right-hand rule)
        let rotated = quat * Vec3::Y;
        // Y should be near 0, Z should be near -1
        assert!(rotated.y.abs() < 0.01, "Y should be ~0, got {}", rotated.y);
        assert!(
            (rotated.z - (-1.0)).abs() < 0.01 || (rotated.z - 1.0).abs() < 0.01,
            "Z should be ~±1, got {}",
            rotated.z
        );
    }

    #[test]
    fn test_animation_parsing() {
        let json = r#"{
            "name": "test",
            "duration": 1.0,
            "keyframes": [
                {
                    "time": 0.0,
                    "pose": {
                        "spine": { "x": 0, "y": 0, "z": 0 }
                    }
                },
                {
                    "time": 0.5,
                    "pose": {
                        "spine": { "x": 45, "y": 0, "z": 0 }
                    }
                }
            ]
        }"#;

        let clip = RotationAnimationClip::from_json(json).unwrap();
        assert_eq!(clip.name, "test");
        assert_eq!(clip.keyframes.len(), 2);
    }
}
