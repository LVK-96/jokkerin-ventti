//! Skeleton vertex data for T-pose stick figure
//!
//! Defines the vertex positions for a humanoid skeleton as line segments.
//! Each pair of consecutive Vec3 values represents a line from point A to point B.

/// Joint identifiers for animation
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JointId {
    Root = 0,
    Hips = 1,
    Chest = 2,
    Neck = 3,
    Head = 4,
    LeftShoulder = 5,
    LeftHand = 6,
    RightShoulder = 7,
    RightHand = 8,
    LeftHip = 9,
    LeftKnee = 10,
    LeftFoot = 11,
    RightHip = 12,
    RightKnee = 13,
    RightFoot = 14,
}

/// Vertex data: start_pos (3), end_pos (3), bone_id (1)
/// Format: 7 floats per bone segment
#[rustfmt::skip]
pub const SKELETON_VERTICES: &[f32] = &[
    // === SPINE ===
    // Hips to Chest
    0.0, 0.5, 0.0,      0.0, 1.0, 0.0,      JointId::Chest as u32 as f32,
    // Chest to Neck
    0.0, 1.0, 0.0,      0.0, 1.2, 0.0,      JointId::Neck as u32 as f32,
    // === HEAD ===
    0.0, 1.35, 0.0,     0.0, 1.35, 0.0,     JointId::Head as u32 as f32,

    // === LEFT ARM ===
    // Neck to Left Shoulder
    0.0, 1.2, 0.0,      -0.15, 1.15, 0.0,   JointId::LeftShoulder as u32 as f32,
    // Left Shoulder to Left Hand
    -0.15, 1.15, 0.0,   -0.4, 0.85, 0.0,    JointId::LeftHand as u32 as f32,

    // === RIGHT ARM ===
    // Neck to Right Shoulder
    0.0, 1.2, 0.0,      0.15, 1.15, 0.0,    JointId::RightShoulder as u32 as f32,
    // Right Shoulder to Right Hand
    0.15, 1.15, 0.0,    0.4, 0.85, 0.0,     JointId::RightHand as u32 as f32,

    // === LEFT LEG ===
    // Hips to Left Hip
    0.0, 0.5, 0.0,      -0.1, 0.45, 0.0,    JointId::LeftHip as u32 as f32,
    // Left Hip to Left Knee
    -0.1, 0.45, 0.0,    -0.25, 0.25, 0.0,   JointId::LeftKnee as u32 as f32,
    // Left Knee to Left Foot
    -0.25, 0.25, 0.0,   -0.45, 0.0, 0.0,    JointId::LeftFoot as u32 as f32,

    // === RIGHT LEG ===
    // Hips to Right Hip
    0.0, 0.5, 0.0,      0.1, 0.45, 0.0,     JointId::RightHip as u32 as f32,
    // Right Hip to Right Knee
    0.1, 0.45, 0.0,     0.25, 0.25, 0.0,    JointId::RightKnee as u32 as f32,
    // Right Knee to Right Foot
    0.25, 0.25, 0.0,    0.45, 0.0, 0.0,     JointId::RightFoot as u32 as f32,
];

/// Number of bone segments in the skeleton
pub const SKELETON_BONE_COUNT: u32 = (SKELETON_VERTICES.len() / 7) as u32;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vertex_count() {
        // 13 segments (Spine: 2, Head: 1, Arms: 2+2, Legs: 3+3)
        // 13 segments * 7 floats = 91 floats
        assert_eq!(SKELETON_VERTICES.len(), 91);
        assert_eq!(SKELETON_BONE_COUNT, 13);
    }

    #[test]
    fn test_vertices_are_reasonable() {
        // All vertices should be within a reasonable bounding box
        for chunk in SKELETON_VERTICES.chunks(7) {
            let x1 = chunk[0];
            let y1 = chunk[1];
            let x2 = chunk[3];
            let y2 = chunk[4];
            let id = chunk[6];

            assert!(x1 >= -1.0 && x1 <= 1.0);
            assert!(y1 >= 0.0 && y1 <= 2.0);
            assert!(x2 >= -1.0 && x2 <= 1.0);
            assert!(y2 >= 0.0 && y2 <= 2.0);
            assert!(id > 0.0);
        }
    }
}
