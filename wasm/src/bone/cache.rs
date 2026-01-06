use super::id::{BONE_HIERARCHY, BoneId};
use glam::{Quat, Vec3A};

/// Dirty flags for lazy forward kinematics evaluation.
/// Uses a bitset where bit i corresponds to BoneId with index i.
#[derive(Debug, Clone, Copy, Default)]
pub struct DirtyFlags(u32);

const fn compute_descendant_masks() -> [u32; BoneId::COUNT] {
    let mut masks = [0u32; BoneId::COUNT];
    let mut i = 0;
    while i < BoneId::COUNT {
        // We are computing mask for bone 'i' (the ancestor)
        // Check every other bone 'j' to see if it is a descendant of 'i'
        let mut mask: u32 = 0;
        let mut j = 0;

        while j < BoneId::COUNT {
            if j == i {
                mask |= 1 << j;
            } else {
                // Check if j is descendant of i
                let mut curr = j;
                let mut is_desc = false;
                let mut depth = 0;

                // Max depth check to ensure termination relative to hierarchy size
                while depth < BoneId::COUNT {
                    if let Some(parent) = BONE_HIERARCHY[curr].parent {
                        if parent.index() == i {
                            is_desc = true;
                            break;
                        }
                        curr = parent.index();
                    } else {
                        break;
                    }
                    depth += 1;
                }

                if is_desc {
                    mask |= 1 << j;
                }
            }
            j += 1;
        }
        masks[i] = mask;
        i += 1;
    }
    masks
}

const DESCENDANT_MASKS: [u32; BoneId::COUNT] = compute_descendant_masks();

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

    /// Check if any bone is dirty
    #[inline]
    pub fn is_any_dirty(&self) -> bool {
        self.0 != 0
    }

    /// Mark a bone and all its children as dirty
    /// Return new flags with a bone and all its children marked as dirty
    pub fn with_marked_dirty(self, bone: BoneId) -> Self {
        // Use precomputed mask for O(1) update
        Self(self.0 | DESCENDANT_MASKS[bone.index()])
    }

    /// Return new flags with dirty flag cleared for a bone
    #[inline]
    pub fn with_cleared(self, bone: BoneId) -> Self {
        Self(self.0 & !(1 << bone.index()))
    }

    /// Return clean flags
    #[inline]
    pub fn cleared() -> Self {
        Self(0)
    }
}

/// Cache for forward kinematics results
#[derive(Debug, Clone)]
pub struct PoseCache {
    /// Cached world transforms (position, rotation)
    /// Lazily computed when needed
    pub world_positions: [Vec3A; BoneId::COUNT],
    pub world_rotations: [Quat; BoneId::COUNT],

    /// Dirty flags for lazy evaluation
    pub dirty: DirtyFlags,
}

impl Default for PoseCache {
    fn default() -> Self {
        Self {
            world_positions: [Vec3A::ZERO; BoneId::COUNT],
            world_rotations: [Quat::IDENTITY; BoneId::COUNT],
            dirty: DirtyFlags::all_dirty(),
        }
    }
}
