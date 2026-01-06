use super::cache::{DirtyFlags, PoseCache};
use super::id::{BoneId, BONE_HIERARCHY};
use crate::skeleton_constants::DEFAULT_PELVIS;
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
        let root_position = Vec3::from(DEFAULT_PELVIS);

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

        // World rotation = parent rotation * local rotation
        let world_rot = parent_rot * local_rot;
        // World position = parent position + rotated bone vector
        let bone_vector = parent_rot * (def.direction.normalize() * def.length);
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

        let mut matrices = [glam::Mat4::IDENTITY; RENDER_BONE_COUNT];

        // Helper macro to get pos
        macro_rules! pos {
            ($id:expr) => {
                cache.world_positions[$id.index()]
            };
        }

        // Current positions
        let pelvis = pos!(BoneId::Pelvis); // Root, should be close to root_position
        let spine1 = pos!(BoneId::Spine1);
        let spine2 = pos!(BoneId::Spine2);
        let spine3 = pos!(BoneId::Spine3);
        let neck = pos!(BoneId::Neck);
        let head = pos!(BoneId::Head);

        let l_collar = pos!(BoneId::LeftCollar);
        let l_shoulder = pos!(BoneId::LeftShoulder);
        let l_elbow = pos!(BoneId::LeftElbow);
        let l_wrist = pos!(BoneId::LeftWrist);

        let r_collar = pos!(BoneId::RightCollar);
        let r_shoulder = pos!(BoneId::RightShoulder);
        let r_elbow = pos!(BoneId::RightElbow);
        let r_wrist = pos!(BoneId::RightWrist);

        let l_hip = pos!(BoneId::LeftHip);
        let l_knee = pos!(BoneId::LeftKnee);
        let l_ankle = pos!(BoneId::LeftAnkle);
        let l_foot = pos!(BoneId::LeftFoot);

        let r_hip = pos!(BoneId::RightHip);
        let r_knee = pos!(BoneId::RightKnee);
        let r_ankle = pos!(BoneId::RightAnkle);
        let r_foot = pos!(BoneId::RightFoot);

        // Cylinders - Must match generate_bind_pose_mesh order in skeleton.rs
        let mut idx = 0;

        // Spine Chain
        matrices[idx] = compute_aligned_matrix(DEFAULT_PELVIS, DEFAULT_SPINE1, pelvis, spine1);
        idx += 1;
        matrices[idx] = compute_aligned_matrix(DEFAULT_SPINE1, DEFAULT_SPINE2, spine1, spine2);
        idx += 1;
        matrices[idx] = compute_aligned_matrix(DEFAULT_SPINE2, DEFAULT_SPINE3, spine2, spine3);
        idx += 1;
        matrices[idx] = compute_aligned_matrix(DEFAULT_SPINE3, DEFAULT_NECK, spine3, neck);
        idx += 1;
        matrices[idx] = compute_aligned_matrix(DEFAULT_NECK, DEFAULT_HEAD, neck, head);
        idx += 1;

        // Left Arm Chain
        matrices[idx] =
            compute_aligned_matrix(DEFAULT_SPINE3, DEFAULT_LEFT_COLLAR, spine3, l_collar);
        idx += 1;
        matrices[idx] = compute_aligned_matrix(
            DEFAULT_LEFT_COLLAR,
            DEFAULT_LEFT_SHOULDER,
            l_collar,
            l_shoulder,
        );
        idx += 1;
        matrices[idx] = compute_aligned_matrix(
            DEFAULT_LEFT_SHOULDER,
            DEFAULT_LEFT_ELBOW,
            l_shoulder,
            l_elbow,
        );
        idx += 1;
        matrices[idx] =
            compute_aligned_matrix(DEFAULT_LEFT_ELBOW, DEFAULT_LEFT_WRIST, l_elbow, l_wrist);
        idx += 1;

        // Right Arm Chain
        matrices[idx] =
            compute_aligned_matrix(DEFAULT_SPINE3, DEFAULT_RIGHT_COLLAR, spine3, r_collar);
        idx += 1;
        matrices[idx] = compute_aligned_matrix(
            DEFAULT_RIGHT_COLLAR,
            DEFAULT_RIGHT_SHOULDER,
            r_collar,
            r_shoulder,
        );
        idx += 1;
        matrices[idx] = compute_aligned_matrix(
            DEFAULT_RIGHT_SHOULDER,
            DEFAULT_RIGHT_ELBOW,
            r_shoulder,
            r_elbow,
        );
        idx += 1;
        matrices[idx] =
            compute_aligned_matrix(DEFAULT_RIGHT_ELBOW, DEFAULT_RIGHT_WRIST, r_elbow, r_wrist);
        idx += 1;

        // Left Leg Chain
        matrices[idx] = compute_aligned_matrix(DEFAULT_PELVIS, DEFAULT_LEFT_HIP, pelvis, l_hip);
        idx += 1;
        matrices[idx] = compute_aligned_matrix(DEFAULT_LEFT_HIP, DEFAULT_LEFT_KNEE, l_hip, l_knee);
        idx += 1;
        matrices[idx] =
            compute_aligned_matrix(DEFAULT_LEFT_KNEE, DEFAULT_LEFT_ANKLE, l_knee, l_ankle);
        idx += 1;
        matrices[idx] =
            compute_aligned_matrix(DEFAULT_LEFT_ANKLE, DEFAULT_LEFT_FOOT, l_ankle, l_foot);
        idx += 1;

        // Right Leg Chain
        matrices[idx] = compute_aligned_matrix(DEFAULT_PELVIS, DEFAULT_RIGHT_HIP, pelvis, r_hip);
        idx += 1;
        matrices[idx] =
            compute_aligned_matrix(DEFAULT_RIGHT_HIP, DEFAULT_RIGHT_KNEE, r_hip, r_knee);
        idx += 1;
        matrices[idx] =
            compute_aligned_matrix(DEFAULT_RIGHT_KNEE, DEFAULT_RIGHT_ANKLE, r_knee, r_ankle);
        idx += 1;
        matrices[idx] =
            compute_aligned_matrix(DEFAULT_RIGHT_ANKLE, DEFAULT_RIGHT_FOOT, r_ankle, r_foot);
        idx += 1;

        // Head Sphere
        matrices[idx] = compute_offset_matrix(DEFAULT_HEAD, head);
        idx += 1;

        // Debug Joints (22 joints)
        for bone_id in BoneId::ALL {
            let def_pos = match bone_id {
                BoneId::Pelvis => DEFAULT_PELVIS,
                BoneId::LeftHip => DEFAULT_LEFT_HIP,
                BoneId::RightHip => DEFAULT_RIGHT_HIP,
                BoneId::Spine1 => DEFAULT_SPINE1,
                BoneId::LeftKnee => DEFAULT_LEFT_KNEE,
                BoneId::RightKnee => DEFAULT_RIGHT_KNEE,
                BoneId::Spine2 => DEFAULT_SPINE2,
                BoneId::LeftAnkle => DEFAULT_LEFT_ANKLE,
                BoneId::RightAnkle => DEFAULT_RIGHT_ANKLE,
                BoneId::Spine3 => DEFAULT_SPINE3,
                BoneId::LeftFoot => DEFAULT_LEFT_FOOT,
                BoneId::RightFoot => DEFAULT_RIGHT_FOOT,
                BoneId::Neck => DEFAULT_NECK,
                BoneId::LeftCollar => DEFAULT_LEFT_COLLAR,
                BoneId::RightCollar => DEFAULT_RIGHT_COLLAR,
                BoneId::Head => DEFAULT_HEAD,
                BoneId::LeftShoulder => DEFAULT_LEFT_SHOULDER,
                BoneId::RightShoulder => DEFAULT_RIGHT_SHOULDER,
                BoneId::LeftElbow => DEFAULT_LEFT_ELBOW,
                BoneId::RightElbow => DEFAULT_RIGHT_ELBOW,
                BoneId::LeftWrist => DEFAULT_LEFT_WRIST,
                BoneId::RightWrist => DEFAULT_RIGHT_WRIST,
            };
            matrices[idx] = compute_offset_matrix(def_pos, pos!(bone_id));
            idx += 1;
        }

        matrices
    }

    /// Interpolate between two poses using spherical linear interpolation (slerp)
    pub fn lerp(a: &RotationPose, b: &RotationPose, t: f32) -> RotationPose {
        let mut result = RotationPose::bind_pose();

        // Lerp root position
        result.root_position = a.root_position.lerp(b.root_position, t);

        // Slerp all rotations with shortest-path correction
        for i in 0..BoneId::COUNT {
            let q_a = a.local_rotations[i];
            let mut q_b = b.local_rotations[i];

            // Ensure we take the shortest path by flipping b if in opposite hemisphere
            if q_a.dot(q_b) < 0.0 {
                q_b = -q_b;
            }

            result.local_rotations[i] = q_a.slerp(q_b, t);
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
