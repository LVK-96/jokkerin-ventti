use glam::Vec3;

// --- Constants ---

/// Default Y position for hips in bind pose (meters)
pub const DEFAULT_HIPS_Y: f32 = 0.55;

/// Hip joint offset from root in X direction (meters)
pub const HIP_OFFSET_X: f32 = 0.02;

/// Hip joint offset from root in Y direction (downward, meters)
pub const HIP_OFFSET_Y: f32 = 0.05;

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
