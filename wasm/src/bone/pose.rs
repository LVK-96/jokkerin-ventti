use glam::{Quat, Vec3};
use std::cell::RefCell;
use super::id::{BoneId, BONE_HIERARCHY, DEFAULT_HIPS_Y, HIP_OFFSET_X, HIP_OFFSET_Y, EPSILON};
use super::cache::{DirtyFlags, PoseCache};

/// Rotation-based pose for animation.
///
/// Each bone stores a local rotation (relative to parent).
/// World positions are computed via forward kinematics and cached internally.
///
/// This struct uses a functional API for state updates (`with_rotation`) but
/// employs interior mutability (`RefCell`) for efficient lazy evaluation.
#[derive(Debug, Clone)]
pub struct RotationPose {
    /// Root position in world space
    pub root_position: Vec3,

    /// Local rotation for each bone (relative to parent)
    pub local_rotations: [Quat; BoneId::COUNT],

    /// Cache for derived world transforms
    pub cache: RefCell<PoseCache>,
}

impl Default for RotationPose {
    fn default() -> Self {
        Self::bind_pose()
    }
}

impl RotationPose {
    /// Create the bind pose (T-pose) with all rotations at identity
    pub fn bind_pose() -> Self {
        let root_position = Vec3::new(0.0, DEFAULT_HIPS_Y, 0.0); // Hips position from skeleton_constants

        Self {
            root_position,
            local_rotations: [Quat::IDENTITY; BoneId::COUNT],
            cache: RefCell::new(PoseCache::default()),
        }
    }

    /// Return a new pose with the specified bone rotation (Functional Set)
    pub fn with_rotation(self, bone: BoneId, rotation: Quat) -> Self {
        let mut new_pose = self;
        if new_pose.local_rotations[bone.index()] != rotation {
            new_pose.local_rotations[bone.index()] = rotation;
            // Mark dirty in the new instance's cache
            let mut cache = new_pose.cache.borrow_mut();
            cache.dirty = cache.dirty.with_marked_dirty(bone);
        }
        new_pose
    }

    /// Return a new pose with the specified root position (Functional Set)
    pub fn with_root_position(self, position: Vec3) -> Self {
        let mut new_pose = self;
        if new_pose.root_position != position {
            new_pose.root_position = position;
            new_pose.cache.borrow_mut().dirty = DirtyFlags::all_dirty();
        }
        new_pose
    }

    /// Mark all bones as needing recomputation
    pub fn with_all_dirty(self) -> Self {
        let new_pose = self;
        new_pose.cache.borrow_mut().dirty = DirtyFlags::all_dirty();
        new_pose
    }

    /// Get world position of a bone's end joint (computes FK if needed)
    pub fn get_position(&self, bone: BoneId) -> Vec3 {
        self.ensure_computed(bone);
        self.cache.borrow().world_positions[bone.index()]
    }

    /// Ensure a bone's world transform is computed
    fn ensure_computed(&self, bone: BoneId) {
        let is_dirty = self.cache.borrow().dirty.is_dirty(bone);
        if !is_dirty {
            return;
        }

        // Compute all ancestors first (they're ordered topologically)
        for ancestor in BoneId::ALL.iter().take(bone.index()) {
            if self.cache.borrow().dirty.is_dirty(*ancestor) {
                self.compute_bone(*ancestor);
            }
        }

        // Compute this bone
        self.compute_bone(bone);
    }

    /// Compute the world transform for a single bone
    fn compute_bone(&self, bone: BoneId) {
        let def = &BONE_HIERARCHY[bone.index()];
        let local_rot = self.local_rotations[bone.index()];

        // We need mutable access to the cache to write results.
        let mut cache = self.cache.borrow_mut();

        let (parent_pos, parent_rot) = if let Some(parent) = def.parent {
            (
                cache.world_positions[parent.index()],
                cache.world_rotations[parent.index()],
            )
        } else {
            // Root bone
            (self.root_position, Quat::IDENTITY)
        };

        // Apply hip offsets for thighs to ensure legs are connected to hips visually
        let parent_pos = if bone == BoneId::LeftThigh {
            parent_pos + parent_rot * Vec3::new(-HIP_OFFSET_X, -HIP_OFFSET_Y, 0.0)
        } else if bone == BoneId::RightThigh {
            parent_pos + parent_rot * Vec3::new(HIP_OFFSET_X, -HIP_OFFSET_Y, 0.0)
        } else {
            parent_pos
        };

        // World rotation = parent rotation * local rotation
        let world_rot = parent_rot * local_rot;

        // World position = parent position + rotated bone vector
        let bone_vector = world_rot * (def.direction.normalize() * def.length);
        let world_pos = parent_pos + bone_vector;

        cache.world_rotations[bone.index()] = world_rot;
        cache.world_positions[bone.index()] = world_pos;
        cache.dirty = cache.dirty.with_cleared(bone);
    }

    /// Force recomputation of all bones (useful after bulk updates)
    pub fn compute_all(&self) {
        for bone in BoneId::ALL {
            self.compute_bone(bone);
        }
        self.cache.borrow_mut().dirty = DirtyFlags::cleared();
    }

    pub fn apply_floor_constraint(self) -> Self {
        // Need to compute to check positions
        self.compute_all();
        use crate::skeleton::BONE_RADIUS;

        let mut min_y = self.root_position.y;
        {
            let cache = self.cache.borrow();
            for i in 0..BoneId::COUNT {
                min_y = min_y.min(cache.world_positions[i].y);
            }

            // Also check hip offsets
            let left_hip_y = self.root_position.y
                + (cache.world_rotations[BoneId::Hips.index()] * Vec3::new(-0.02, -0.05, 0.0)).y;
            let right_hip_y = self.root_position.y
                + (cache.world_rotations[BoneId::Hips.index()] * Vec3::new(0.02, -0.05, 0.0)).y;
            min_y = min_y.min(left_hip_y).min(right_hip_y);
        }

        let mut new_pose = self;
        if min_y < BONE_RADIUS {
            new_pose.root_position.y += BONE_RADIUS - min_y;
            new_pose = new_pose.with_all_dirty();
            // Ensure consistency immediately
            new_pose.compute_all();
        }
        new_pose
    }

    /// Convert to the old Skeleton format for rendering compatibility
    pub fn to_skeleton(&self) -> crate::skeleton::Skeleton {
        self.ensure_computed(BoneId::LeftShin); // Hack to trigger chain? Better just compute_all
        self.compute_all();

        use glam::Vec3A;
        let cache = self.cache.borrow();

        // Map rotation pose joints to skeleton positions
        crate::skeleton::Skeleton {
            hips: Vec3A::from(self.root_position),
            neck: Vec3A::from(cache.world_positions[BoneId::Spine.index()]),
            head: Vec3A::from(cache.world_positions[BoneId::Head.index()]),
            left_shoulder: Vec3A::from(cache.world_positions[BoneId::LeftShoulder.index()]),
            left_elbow: Vec3A::from(cache.world_positions[BoneId::LeftUpperArm.index()]),
            left_hand: Vec3A::from(cache.world_positions[BoneId::LeftForearm.index()]),
            right_shoulder: Vec3A::from(cache.world_positions[BoneId::RightShoulder.index()]),
            right_elbow: Vec3A::from(cache.world_positions[BoneId::RightUpperArm.index()]),
            right_hand: Vec3A::from(cache.world_positions[BoneId::RightForearm.index()]),

            // Calculate actual hip positions based on root rotation + offset
            left_hip: Vec3A::from(
                self.root_position
                    + cache.world_rotations[BoneId::Hips.index()] * Vec3::new(-0.02, -0.05, 0.0),
            ),
            left_knee: Vec3A::from(cache.world_positions[BoneId::LeftThigh.index()]),
            left_foot: Vec3A::from(cache.world_positions[BoneId::LeftShin.index()]),

            right_hip: Vec3A::from(
                self.root_position
                    + cache.world_rotations[BoneId::Hips.index()] * Vec3::new(0.02, -0.05, 0.0),
            ),
            right_knee: Vec3A::from(cache.world_positions[BoneId::RightThigh.index()]),
            right_foot: Vec3A::from(cache.world_positions[BoneId::RightShin.index()]),
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

        // Mark all dirty
        result.cache.borrow_mut().dirty = DirtyFlags::all_dirty();

        result
    }

    pub const IK_ITERATIONS: usize = 10;
    pub const IK_TOLERANCE: f32 = 0.001;

    /// Apply IK to a chain of bones to reach a target position.
    /// Returns modified self (Functional Chain).
    ///
    /// # Arguments
    /// * `chain` - List of bone IDs in the chain (parent to child/end-effector)
    /// * `target` - Target world position for the end effector
    pub fn apply_ik(self, chain: &[BoneId], target: Vec3) -> Self {
        if chain.is_empty() {
            return self;
        }

        // 1. Gather current world positions and bone lengths
        let mut joints = Vec::with_capacity(chain.len() + 1);
        let mut lengths = Vec::with_capacity(chain.len());

        // Start position (parent of first bone in chain)
        let start_bone = chain[0];

        let root_pos = if let Some(parent) = BONE_HIERARCHY[start_bone.index()].parent {
            self.get_position(parent)
        } else {
            self.root_position
        };
        joints.push(root_pos);

        for &bone in chain {
            joints.push(self.get_position(bone));
            lengths.push(BONE_HIERARCHY[bone.index()].length);
        }

        // 2. Solve IK (FABRIK)
        let solved_joints = crate::ik::solve_fabrik(
            joints,
            &lengths,
            target,
            Self::IK_ITERATIONS,
            Self::IK_TOLERANCE,
        );

        // 3. Update local rotations
        let mut current_parent_rot = if let Some(parent) = BONE_HIERARCHY[start_bone.index()].parent
        {
            self.ensure_computed(parent);
            self.cache.borrow().world_rotations[parent.index()]
        } else {
            Quat::IDENTITY
        };

        let mut new_pose = self;
        for (i, &bone) in chain.iter().enumerate() {
            let def = &BONE_HIERARCHY[bone.index()];

            let start_pos = solved_joints[i];
            let end_pos = solved_joints[i + 1];
            let target_vec = end_pos - start_pos;

            if target_vec.length_squared() < EPSILON {
                continue;
            }

            let target_dir_local = current_parent_rot.inverse() * target_vec.normalize();
            let default_dir = def.direction.normalize();

            let delta_rot = Quat::from_rotation_arc(default_dir, target_dir_local);

            new_pose = new_pose.with_rotation(bone, delta_rot.normalize());
            new_pose.compute_bone(bone);
            current_parent_rot = new_pose.cache.borrow().world_rotations[bone.index()];
        }

        new_pose
    }
}
