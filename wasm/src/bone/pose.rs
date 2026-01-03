use super::cache::{DirtyFlags, PoseCache};
use super::id::{BoneId, BONE_HIERARCHY, DEFAULT_HIPS_Y, HIP_OFFSET_X, HIP_OFFSET_Y};
use crate::EPSILON;
use glam::{Quat, Vec3, Vec3A};
use std::cell::RefCell;

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
        Vec3::from(self.cache.borrow().world_positions[bone.index()])
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
                Vec3::from(cache.world_positions[parent.index()]),
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
        cache.world_positions[bone.index()] = Vec3A::from(world_pos);
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
        if self.cache.borrow().dirty.is_any_dirty() {
            self.compute_all();
        }
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

    /// Compute all bone matrices for skinning
    /// Returns [Mat4; RENDER_BONE_COUNT]
    pub fn compute_bone_matrices(&self) -> [glam::Mat4; crate::skeleton::RENDER_BONE_COUNT] {
        self.compute_all();
        let cache = self.cache.borrow();

        use crate::skeleton::{compute_aligned_matrix, compute_offset_matrix, RENDER_BONE_COUNT};
        use crate::skeleton_constants::*;
        use glam::Vec3A;

        let mut matrices = [glam::Mat4::IDENTITY; RENDER_BONE_COUNT];

        // Current positions (stored as Vec3A in cache)
        let hips = Vec3A::from(self.root_position);
        let neck = cache.world_positions[BoneId::Spine.index()];
        let head = cache.world_positions[BoneId::Head.index()];
        let left_shoulder = cache.world_positions[BoneId::LeftShoulder.index()];
        let left_elbow = cache.world_positions[BoneId::LeftUpperArm.index()];
        let left_hand = cache.world_positions[BoneId::LeftForearm.index()];
        let right_shoulder = cache.world_positions[BoneId::RightShoulder.index()];
        let right_elbow = cache.world_positions[BoneId::RightUpperArm.index()];
        let right_hand = cache.world_positions[BoneId::RightForearm.index()];

        // Hip offsets
        let left_hip_offset =
            cache.world_rotations[BoneId::Hips.index()] * Vec3::new(-0.02, -0.05, 0.0);
        let right_hip_offset =
            cache.world_rotations[BoneId::Hips.index()] * Vec3::new(0.02, -0.05, 0.0);

        let left_hip = Vec3A::from(self.root_position + left_hip_offset);
        let right_hip = Vec3A::from(self.root_position + right_hip_offset);
        let left_knee = cache.world_positions[BoneId::LeftThigh.index()];
        let left_foot = cache.world_positions[BoneId::LeftShin.index()];
        let right_knee = cache.world_positions[BoneId::RightThigh.index()];
        let right_foot = cache.world_positions[BoneId::RightShin.index()];

        // Cylinders
        matrices[0] = compute_aligned_matrix(DEFAULT_HIPS, DEFAULT_NECK, hips, neck);
        matrices[1] =
            compute_aligned_matrix(DEFAULT_NECK, DEFAULT_LEFT_SHOULDER, neck, left_shoulder);
        matrices[2] = compute_aligned_matrix(
            DEFAULT_LEFT_SHOULDER,
            DEFAULT_LEFT_ELBOW,
            left_shoulder,
            left_elbow,
        );
        matrices[3] =
            compute_aligned_matrix(DEFAULT_LEFT_ELBOW, DEFAULT_LEFT_HAND, left_elbow, left_hand);
        matrices[4] =
            compute_aligned_matrix(DEFAULT_NECK, DEFAULT_RIGHT_SHOULDER, neck, right_shoulder);
        matrices[5] = compute_aligned_matrix(
            DEFAULT_RIGHT_SHOULDER,
            DEFAULT_RIGHT_ELBOW,
            right_shoulder,
            right_elbow,
        );
        matrices[6] = compute_aligned_matrix(
            DEFAULT_RIGHT_ELBOW,
            DEFAULT_RIGHT_HAND,
            right_elbow,
            right_hand,
        );

        matrices[7] = compute_aligned_matrix(DEFAULT_HIPS, DEFAULT_LEFT_HIP, hips, left_hip);
        matrices[8] =
            compute_aligned_matrix(DEFAULT_LEFT_HIP, DEFAULT_LEFT_KNEE, left_hip, left_knee);
        matrices[9] =
            compute_aligned_matrix(DEFAULT_LEFT_KNEE, DEFAULT_LEFT_FOOT, left_knee, left_foot);

        matrices[10] = compute_aligned_matrix(DEFAULT_HIPS, DEFAULT_RIGHT_HIP, hips, right_hip);
        matrices[11] =
            compute_aligned_matrix(DEFAULT_RIGHT_HIP, DEFAULT_RIGHT_KNEE, right_hip, right_knee);
        matrices[12] = compute_aligned_matrix(
            DEFAULT_RIGHT_KNEE,
            DEFAULT_RIGHT_FOOT,
            right_knee,
            right_foot,
        );

        // Head Sphere
        matrices[13] = compute_offset_matrix(DEFAULT_HEAD, head);

        // Debug joints
        matrices[14] = compute_offset_matrix(DEFAULT_HIPS, hips);
        matrices[15] = compute_offset_matrix(DEFAULT_NECK, neck);
        matrices[16] = compute_offset_matrix(DEFAULT_LEFT_SHOULDER, left_shoulder);
        matrices[17] = compute_offset_matrix(DEFAULT_LEFT_ELBOW, left_elbow);
        matrices[18] = compute_offset_matrix(DEFAULT_LEFT_HAND, left_hand);
        matrices[19] = compute_offset_matrix(DEFAULT_RIGHT_SHOULDER, right_shoulder);
        matrices[20] = compute_offset_matrix(DEFAULT_RIGHT_ELBOW, right_elbow);
        matrices[21] = compute_offset_matrix(DEFAULT_RIGHT_HAND, right_hand);
        matrices[22] = compute_offset_matrix(DEFAULT_LEFT_HIP, left_hip);
        matrices[23] = compute_offset_matrix(DEFAULT_LEFT_KNEE, left_knee);
        matrices[24] = compute_offset_matrix(DEFAULT_LEFT_FOOT, left_foot);
        matrices[25] = compute_offset_matrix(DEFAULT_RIGHT_HIP, right_hip);
        matrices[26] = compute_offset_matrix(DEFAULT_RIGHT_KNEE, right_knee);
        matrices[27] = compute_offset_matrix(DEFAULT_RIGHT_FOOT, right_foot);

        matrices
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
