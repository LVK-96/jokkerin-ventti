use glam::{Quat, Vec3};
use super::id::{BoneId, BONE_HIERARCHY};

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

    /// Check if any bone is dirty
    #[inline]
    pub fn is_any_dirty(&self) -> bool {
        self.0 != 0
    }

    /// Mark a bone and all its children as dirty
    /// Return new flags with a bone and all its children marked as dirty
    pub fn with_marked_dirty(self, bone: BoneId) -> Self {
        let mut new_flags = self;
        // Mark this bone
        new_flags.0 |= 1 << bone.index();

        // Mark all children (bones that have this as ancestor)
        for child in BoneId::ALL.iter().skip(bone.index() + 1) {
            if Self::is_descendant_of(*child, bone) {
                new_flags.0 |= 1 << child.index();
            }
        }
        new_flags
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

/// Cache for forward kinematics results
#[derive(Debug, Clone)]
pub struct PoseCache {
    /// Cached world transforms (position, rotation)
    /// Lazily computed when needed
    pub world_positions: [Vec3; BoneId::COUNT],
    pub world_rotations: [Quat; BoneId::COUNT],

    /// Dirty flags for lazy evaluation
    pub dirty: DirtyFlags,
}

impl Default for PoseCache {
    fn default() -> Self {
        Self {
            world_positions: [Vec3::ZERO; BoneId::COUNT],
            world_rotations: [Quat::IDENTITY; BoneId::COUNT],
            dirty: DirtyFlags::all_dirty(),
        }
    }
}
