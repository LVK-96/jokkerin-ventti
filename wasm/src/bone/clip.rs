use glam::Quat;
use super::id::BoneId;
use super::pose::RotationPose;
use serde::{Deserialize, Serialize};

// ============================================================================
// Animation System
// ============================================================================

/// Euler angles in degrees for JSON authoring (more intuitive than quaternions)
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
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
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
    pub neck: Option<EulerAngles>,

    #[serde(default)]
    pub head: Option<EulerAngles>,

    #[serde(default)]
    pub left_shoulder: Option<EulerAngles>,

    #[serde(default)]
    pub left_upper_arm: Option<EulerAngles>,

    #[serde(default)]
    pub left_forearm: Option<EulerAngles>,

    #[serde(default)]
    pub right_shoulder: Option<EulerAngles>,

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
            pose.root_position = glam::Vec3::new(x, y, z);
        }

        // Apply rotations for each bone if specified
        if let Some(euler) = self.hips {
            pose.local_rotations[BoneId::Hips.index()] = euler.to_quat();
        }
        if let Some(euler) = self.spine {
            pose.local_rotations[BoneId::Spine.index()] = euler.to_quat();
        }
        if let Some(euler) = self.neck {
            pose.local_rotations[BoneId::Neck.index()] = euler.to_quat();
        }
        if let Some(euler) = self.head {
            pose.local_rotations[BoneId::Head.index()] = euler.to_quat();
        }
        if let Some(euler) = self.left_shoulder {
            pose.local_rotations[BoneId::LeftShoulder.index()] = euler.to_quat();
        }
        if let Some(euler) = self.left_upper_arm {
            pose.local_rotations[BoneId::LeftUpperArm.index()] = euler.to_quat();
        }
        if let Some(euler) = self.left_forearm {
            pose.local_rotations[BoneId::LeftForearm.index()] = euler.to_quat();
        }
        if let Some(euler) = self.right_shoulder {
            pose.local_rotations[BoneId::RightShoulder.index()] = euler.to_quat();
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

    /// Create from RotationPose
    pub fn from_pose(pose: &RotationPose) -> Self {
        let mut json = RotationPoseJson::default();

        // Only include root position if non-zero
        if pose.root_position.length_squared() > 1e-6 {
            json.root_position = Some(pose.root_position.to_array());
        }

        let is_identity = |q: Quat| q.angle_between(Quat::IDENTITY) < 1e-4;

        if !is_identity(pose.local_rotations[BoneId::Hips.index()]) {
            json.hips = Some(EulerAngles::from_quat(
                pose.local_rotations[BoneId::Hips.index()],
            ));
        }
        if !is_identity(pose.local_rotations[BoneId::Spine.index()]) {
            json.spine = Some(EulerAngles::from_quat(
                pose.local_rotations[BoneId::Spine.index()],
            ));
        }
        if !is_identity(pose.local_rotations[BoneId::Neck.index()]) {
            json.neck = Some(EulerAngles::from_quat(
                pose.local_rotations[BoneId::Neck.index()],
            ));
        }
        if !is_identity(pose.local_rotations[BoneId::Head.index()]) {
            json.head = Some(EulerAngles::from_quat(
                pose.local_rotations[BoneId::Head.index()],
            ));
        }
        if !is_identity(pose.local_rotations[BoneId::LeftShoulder.index()]) {
            json.left_shoulder = Some(EulerAngles::from_quat(
                pose.local_rotations[BoneId::LeftShoulder.index()],
            ));
        }
        if !is_identity(pose.local_rotations[BoneId::LeftUpperArm.index()]) {
            json.left_upper_arm = Some(EulerAngles::from_quat(
                pose.local_rotations[BoneId::LeftUpperArm.index()],
            ));
        }
        if !is_identity(pose.local_rotations[BoneId::LeftForearm.index()]) {
            json.left_forearm = Some(EulerAngles::from_quat(
                pose.local_rotations[BoneId::LeftForearm.index()],
            ));
        }
        if !is_identity(pose.local_rotations[BoneId::RightShoulder.index()]) {
            json.right_shoulder = Some(EulerAngles::from_quat(
                pose.local_rotations[BoneId::RightShoulder.index()],
            ));
        }
        if !is_identity(pose.local_rotations[BoneId::RightUpperArm.index()]) {
            json.right_upper_arm = Some(EulerAngles::from_quat(
                pose.local_rotations[BoneId::RightUpperArm.index()],
            ));
        }
        if !is_identity(pose.local_rotations[BoneId::RightForearm.index()]) {
            json.right_forearm = Some(EulerAngles::from_quat(
                pose.local_rotations[BoneId::RightForearm.index()],
            ));
        }
        if !is_identity(pose.local_rotations[BoneId::LeftThigh.index()]) {
            json.left_thigh = Some(EulerAngles::from_quat(
                pose.local_rotations[BoneId::LeftThigh.index()],
            ));
        }
        if !is_identity(pose.local_rotations[BoneId::LeftShin.index()]) {
            json.left_shin = Some(EulerAngles::from_quat(
                pose.local_rotations[BoneId::LeftShin.index()],
            ));
        }
        if !is_identity(pose.local_rotations[BoneId::RightThigh.index()]) {
            json.right_thigh = Some(EulerAngles::from_quat(
                pose.local_rotations[BoneId::RightThigh.index()],
            ));
        }
        if !is_identity(pose.local_rotations[BoneId::RightShin.index()]) {
            json.right_shin = Some(EulerAngles::from_quat(
                pose.local_rotations[BoneId::RightShin.index()],
            ));
        }

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
#[derive(Debug, Clone, Deserialize, Serialize)]
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
