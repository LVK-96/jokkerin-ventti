use super::id::BoneId;
use super::pose::RotationPose;
use glam::Quat;
use half::f16; // Note: We use the 'half' crate because the native WASM target does not support f16
use serde::{Deserialize, Serialize};

// ============================================================================
// Binary Format Helpers
// ============================================================================

/// Convert Q1.15 signed fixed-point to f32
/// Q1.15 has 1 sign bit and 15 fractional bits, range [-1.0, 1.0)
fn q15_to_f32(bytes: [u8; 2]) -> f32 {
    let val = i16::from_le_bytes(bytes);
    val as f32 / 32767.0
}

// ============================================================================
// Animation System
// ============================================================================

/// Euler angles for JSON representation
#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize)]
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
}

/// Quaternion representation for JSON
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct QuaternionJson {
    pub w: f32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl QuaternionJson {
    pub fn to_quat(&self) -> Quat {
        Quat::from_xyzw(self.x, self.y, self.z, self.w)
    }
    pub fn from_quat(q: Quat) -> Self {
        let (x, y, z, w) = q.into();
        Self { w, x, y, z }
    }
}

/// A bone rotation can be specified as Euler angles or a Quaternion
/// NOTE: Quaternion MUST be listed first in the enum because with #[serde(untagged)],
/// serde tries variants in order. Since EulerAngles has all fields with #[serde(default)],
/// it would incorrectly match quaternion JSON by ignoring the 'w' field.
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(untagged)]
pub enum BoneRotation {
    Quaternion(QuaternionJson),
    Euler(EulerAngles),
}

impl BoneRotation {
    pub fn to_quat(&self) -> Quat {
        match self {
            BoneRotation::Euler(e) => e.to_quat(),
            BoneRotation::Quaternion(q) => q.to_quat(),
        }
    }
}

/// A single keyframe's bone rotations in JSON format
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct RotationPoseJson {
    /// Root position override (optional)
    #[serde(default, rename = "rp")]
    pub root_position: Option<[f32; 3]>,

    #[serde(default, rename = "p")]
    pub pelvis: Option<BoneRotation>,
    #[serde(default, rename = "lh")]
    pub l_hip: Option<BoneRotation>,
    #[serde(default, rename = "rh")]
    pub r_hip: Option<BoneRotation>,
    #[serde(default, rename = "s1")]
    pub spine1: Option<BoneRotation>,
    #[serde(default, rename = "lk")]
    pub l_knee: Option<BoneRotation>,
    #[serde(default, rename = "rk")]
    pub r_knee: Option<BoneRotation>,
    #[serde(default, rename = "s2")]
    pub spine2: Option<BoneRotation>,
    #[serde(default, rename = "la")]
    pub l_ankle: Option<BoneRotation>,
    #[serde(default, rename = "ra")]
    pub r_ankle: Option<BoneRotation>,
    #[serde(default, rename = "s3")]
    pub spine3: Option<BoneRotation>,
    #[serde(default, rename = "lf")]
    pub l_foot: Option<BoneRotation>,
    #[serde(default, rename = "rf")]
    pub r_foot: Option<BoneRotation>,
    #[serde(default, rename = "n")]
    pub neck: Option<BoneRotation>,
    #[serde(default, rename = "lc")]
    pub l_collar: Option<BoneRotation>,
    #[serde(default, rename = "rc")]
    pub r_collar: Option<BoneRotation>,
    #[serde(default, rename = "h")]
    pub head: Option<BoneRotation>,
    #[serde(default, rename = "ls")]
    pub l_shoulder: Option<BoneRotation>,
    #[serde(default, rename = "rs")]
    pub r_shoulder: Option<BoneRotation>,
    #[serde(default, rename = "le")]
    pub l_elbow: Option<BoneRotation>,
    #[serde(default, rename = "re")]
    pub r_elbow: Option<BoneRotation>,
    #[serde(default, rename = "lw")]
    pub l_wrist: Option<BoneRotation>,
    #[serde(default, rename = "rw")]
    pub r_wrist: Option<BoneRotation>,
}

impl RotationPoseJson {
    /// Convert JSON pose to RotationPose
    pub fn to_rotation_pose(&self) -> RotationPose {
        let mut pose = RotationPose::bind_pose();

        // Apply root position if specified
        if let Some([x, y, z]) = self.root_position {
            pose.root_position = glam::Vec3::new(x, y, z);
        }

        // Apply rotations for each bone if specified
        if let Some(rot) = self.pelvis {
            pose.local_rotations[BoneId::Pelvis.index()] = rot.to_quat();
        }
        if let Some(rot) = self.l_hip {
            pose.local_rotations[BoneId::LeftHip.index()] = rot.to_quat();
        }
        if let Some(rot) = self.r_hip {
            pose.local_rotations[BoneId::RightHip.index()] = rot.to_quat();
        }
        if let Some(rot) = self.spine1 {
            pose.local_rotations[BoneId::Spine1.index()] = rot.to_quat();
        }
        if let Some(rot) = self.l_knee {
            pose.local_rotations[BoneId::LeftKnee.index()] = rot.to_quat();
        }
        if let Some(rot) = self.r_knee {
            pose.local_rotations[BoneId::RightKnee.index()] = rot.to_quat();
        }
        if let Some(rot) = self.spine2 {
            pose.local_rotations[BoneId::Spine2.index()] = rot.to_quat();
        }
        if let Some(rot) = self.l_ankle {
            pose.local_rotations[BoneId::LeftAnkle.index()] = rot.to_quat();
        }
        if let Some(rot) = self.r_ankle {
            pose.local_rotations[BoneId::RightAnkle.index()] = rot.to_quat();
        }
        if let Some(rot) = self.spine3 {
            pose.local_rotations[BoneId::Spine3.index()] = rot.to_quat();
        }
        if let Some(rot) = self.l_foot {
            pose.local_rotations[BoneId::LeftFoot.index()] = rot.to_quat();
        }
        if let Some(rot) = self.r_foot {
            pose.local_rotations[BoneId::RightFoot.index()] = rot.to_quat();
        }
        if let Some(rot) = self.neck {
            pose.local_rotations[BoneId::Neck.index()] = rot.to_quat();
        }
        if let Some(rot) = self.l_collar {
            pose.local_rotations[BoneId::LeftCollar.index()] = rot.to_quat();
        }
        if let Some(rot) = self.r_collar {
            pose.local_rotations[BoneId::RightCollar.index()] = rot.to_quat();
        }
        if let Some(rot) = self.head {
            pose.local_rotations[BoneId::Head.index()] = rot.to_quat();
        }
        if let Some(rot) = self.l_shoulder {
            pose.local_rotations[BoneId::LeftShoulder.index()] = rot.to_quat();
        }
        if let Some(rot) = self.r_shoulder {
            pose.local_rotations[BoneId::RightShoulder.index()] = rot.to_quat();
        }
        if let Some(rot) = self.l_elbow {
            pose.local_rotations[BoneId::LeftElbow.index()] = rot.to_quat();
        }
        if let Some(rot) = self.r_elbow {
            pose.local_rotations[BoneId::RightElbow.index()] = rot.to_quat();
        }
        if let Some(rot) = self.l_wrist {
            pose.local_rotations[BoneId::LeftWrist.index()] = rot.to_quat();
        }
        if let Some(rot) = self.r_wrist {
            pose.local_rotations[BoneId::RightWrist.index()] = rot.to_quat();
        }

        pose
    }

    /// Create from RotationPose
    pub fn from_pose(pose: &RotationPose) -> Self {
        let mut json = RotationPoseJson::default();

        // Only include root position if non-zero
        if pose.root_position.length_squared() > 1e-6 {
            json.root_position = Some(pose.root_position.to_array());
        }

        let is_identity = |q: Quat| q.angle_between(Quat::IDENTITY) < 1e-4;

        macro_rules! map_bone {
            ($id:expr, $field:ident) => {
                if !is_identity(pose.local_rotations[$id.index()]) {
                    let q = pose.local_rotations[$id.index()];
                    json.$field = Some(BoneRotation::Quaternion(QuaternionJson::from_quat(q)));
                }
            };
        }

        map_bone!(BoneId::Pelvis, pelvis);
        map_bone!(BoneId::LeftHip, l_hip);
        map_bone!(BoneId::RightHip, r_hip);
        map_bone!(BoneId::Spine1, spine1);
        map_bone!(BoneId::LeftKnee, l_knee);
        map_bone!(BoneId::RightKnee, r_knee);
        map_bone!(BoneId::Spine2, spine2);
        map_bone!(BoneId::LeftAnkle, l_ankle);
        map_bone!(BoneId::RightAnkle, r_ankle);
        map_bone!(BoneId::Spine3, spine3);
        map_bone!(BoneId::LeftFoot, l_foot);
        map_bone!(BoneId::RightFoot, r_foot);
        map_bone!(BoneId::Neck, neck);
        map_bone!(BoneId::LeftCollar, l_collar);
        map_bone!(BoneId::RightCollar, r_collar);
        map_bone!(BoneId::Head, head);
        map_bone!(BoneId::LeftShoulder, l_shoulder);
        map_bone!(BoneId::RightShoulder, r_shoulder);
        map_bone!(BoneId::LeftElbow, l_elbow);
        map_bone!(BoneId::RightElbow, r_elbow);
        map_bone!(BoneId::LeftWrist, l_wrist);
        map_bone!(BoneId::RightWrist, r_wrist);

        json
    }
}

/// A keyframe in a rotation-based animation
#[derive(Debug, Clone)]
pub struct RotationKeyframe {
    pub time: f32,
    pub pose: RotationPose,
}

/// JSON format for keyframe
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RotationKeyframeJson {
    #[serde(rename = "t")]
    pub time: f32,
    #[serde(rename = "p")]
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
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RotationAnimationClipJson {
    #[serde(rename = "$schema", default, skip_deserializing)]
    pub schema: Option<String>,
    #[serde(default = "default_version", rename = "v")]
    pub version: u32,
    #[serde(rename = "n")]
    pub name: String,
    #[serde(rename = "d")]
    pub duration: f32,
    #[serde(rename = "kf")]
    pub keyframes: Vec<RotationKeyframeJson>,
}

fn default_version() -> u32 {
    2
}

impl RotationAnimationClip {
    /// Parse from JSON string
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        let clip_json: RotationAnimationClipJson = serde_json::from_str(json)?;

        let keyframes: Vec<RotationKeyframe> = clip_json
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

    /// Parse from binary format
    ///
    /// Binary format:
    /// - Header: u16 keyframe_count, f16 duration
    /// - Per keyframe: 22 bones * 4 Q1.15 values + 3 f16 root position (182 bytes)
    pub fn from_binary(data: &[u8], name: String) -> Result<Self, &'static str> {
        if data.len() < 8 {
            return Err("Binary data too short for header");
        }

        // 1. Read Header (8 bytes)
        let keyframe_count = u16::from_le_bytes([data[0], data[1]]) as usize;
        let duration = f16::from_le_bytes([data[2], data[3]]).to_f32();
        let dynamic_mask = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);

        let mut offset = 8;

        // 2. Read Base Data (Header extension)
        // Base Root (6 bytes)
        if data.len() < offset + 6 {
            return Err("Binary data too short for base root");
        }
        let base_rx = f16::from_le_bytes([data[offset], data[offset + 1]]).to_f32();
        let base_ry = f16::from_le_bytes([data[offset + 2], data[offset + 3]]).to_f32();
        let base_rz = f16::from_le_bytes([data[offset + 4], data[offset + 5]]).to_f32();
        let base_root = glam::Vec3::new(base_rx, base_ry, base_rz);
        offset += 6;

        // Base Rotations (22 bones * 6 bytes = 132 bytes)
        if data.len() < offset + 132 {
            return Err("Binary data too short for base rotations");
        }
        let mut base_rotations = [Quat::IDENTITY; BoneId::COUNT];
        for i in 0..BoneId::COUNT {
            let x = q15_to_f32([data[offset], data[offset + 1]]);
            let y = q15_to_f32([data[offset + 2], data[offset + 3]]);
            let z = q15_to_f32([data[offset + 4], data[offset + 5]]);
            offset += 6;

            // Reconstruct W: w^2 + x^2 + y^2 + z^2 = 1.0
            let sum_sq = x * x + y * y + z * z;
            let w = (1.0 - sum_sq).max(0.0).sqrt();
            base_rotations[i] = Quat::from_xyzw(x, y, z, w).normalize();
        }

        // 3. Read Dynamic Keyframe Data
        let mut keyframes = Vec::with_capacity(keyframe_count);

        for i in 0..keyframe_count {
            let mut pose = RotationPose::bind_pose();
            pose.root_position = base_root;
            pose.local_rotations = base_rotations;

            // Read dynamic rotations (3 components each)
            for bone_idx in 0..BoneId::COUNT {
                if dynamic_mask & (1 << bone_idx) != 0 {
                    if data.len() < offset + 6 {
                        return Err("Binary data truncated in dynamic rotations");
                    }
                    let x = q15_to_f32([data[offset], data[offset + 1]]);
                    let y = q15_to_f32([data[offset + 2], data[offset + 3]]);
                    let z = q15_to_f32([data[offset + 4], data[offset + 5]]);
                    offset += 6;

                    let sum_sq = x * x + y * y + z * z;
                    let w = (1.0 - sum_sq).max(0.0).sqrt();
                    pose.local_rotations[bone_idx] = Quat::from_xyzw(x, y, z, w).normalize();
                }
            }

            // Read dynamic root position
            if dynamic_mask & (1 << 22) != 0 {
                if data.len() < offset + 6 {
                    return Err("Binary data truncated in dynamic root position");
                }
                let rx = f16::from_le_bytes([data[offset], data[offset + 1]]).to_f32();
                let ry = f16::from_le_bytes([data[offset + 2], data[offset + 3]]).to_f32();
                let rz = f16::from_le_bytes([data[offset + 4], data[offset + 5]]).to_f32();
                pose.root_position = glam::Vec3::new(rx, ry, rz);
                offset += 6;
            }

            // Keyframes are evenly spaced over duration
            let time = if keyframe_count > 1 {
                duration * (i as f32) / ((keyframe_count - 1) as f32)
            } else {
                0.0
            };

            keyframes.push(RotationKeyframe { time, pose });
        }

        Ok(Self {
            name,
            duration,
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
                pose: RotationPoseJson::from_pose(&kf.pose),
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
