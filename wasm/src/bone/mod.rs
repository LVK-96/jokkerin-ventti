pub mod id;
pub mod cache;
pub mod pose;
pub mod clip;

pub use id::*;
pub use cache::*;
pub use pose::*;
pub use clip::*;

#[cfg(test)]
mod tests {
    use super::*;

    use glam::{Vec3, Quat};

    #[test]
    fn test_bind_pose_positions() {
        let pose = RotationPose::bind_pose();
        // Force computation to ensure world positions are ready
        pose.compute_all();

        let hips_pos = pose.get_position(BoneId::Hips);

        // Hips should be at DEFAULT_HIPS_Y
        assert!(
            (hips_pos.y - DEFAULT_HIPS_Y).abs() < EPSILON,
            "Hips Y should be {}, got {}",
            DEFAULT_HIPS_Y,
            hips_pos.y
        );

        // Head should be above hips
        let head_pos = pose.get_position(BoneId::Head);
        assert!(head_pos.y > hips_pos.y);

        // Feet should be near ground
        let left_foot_pos = pose.get_position(BoneId::LeftShin);
        let right_foot_pos = pose.get_position(BoneId::RightShin);

        assert!(left_foot_pos.y < 0.1);
        assert!(right_foot_pos.y < 0.1);
    }

    #[test]
    fn test_floor_constraint() {
        let pose = RotationPose::bind_pose();
        // Move root way below floor
        let mut pose = pose.with_root_position(Vec3::new(0.0, -2.0, 0.0));
        pose.compute_all();

        // Verify it's below floor
        assert!(pose.get_position(BoneId::Hips).y < 0.0);

        // Apply constraint
        pose = pose.apply_floor_constraint();

        // Should be lifted
        // The lowest point should be at BONE_RADIUS (approx 0.05)
        let hips_y = pose.get_position(BoneId::Hips).y;
        assert!(hips_y > 0.0, "Hips should be above floor, got {}", hips_y);

        // More precise check: calculate min y of all bones
        let mut min_y = f32::MAX;
        for bone in BoneId::ALL {
            min_y = min_y.min(pose.get_position(bone).y);
        }
        // crate::skeleton::BONE_RADIUS is usually 0.05.
        // We ensure that we are at least non-negative.
        assert!(min_y >= 0.0, "Lowest bone should be above 0, got {}", min_y);
    }

    #[test]
    fn test_ik_preserves_chain_lengths() {
        let pose = RotationPose::bind_pose();
        let chain = [
            BoneId::LeftShoulder,
            BoneId::LeftUpperArm,
            BoneId::LeftForearm,
        ];

        // Get initial lengths
        let pos_shoulder = pose.get_position(BoneId::LeftShoulder);
        let pos_elbow = pose.get_position(BoneId::LeftUpperArm);
        let pos_hand = pose.get_position(BoneId::LeftForearm);

        let len_upper = pos_shoulder.distance(pos_elbow);
        let len_forearm = pos_elbow.distance(pos_hand);

        // Apply IK to a new target
        let target = Vec3::new(0.5, 0.5, 0.5);
        let pose = pose.apply_ik(&chain, target);

        let new_pos_shoulder = pose.get_position(BoneId::LeftShoulder);
        let new_pos_elbow = pose.get_position(BoneId::LeftUpperArm);
        let new_pos_hand = pose.get_position(BoneId::LeftForearm);

        let new_len_upper = new_pos_shoulder.distance(new_pos_elbow);
        let new_len_forearm = new_pos_elbow.distance(new_pos_hand);

        assert!(
            (len_upper - new_len_upper).abs() < 1e-4,
            "Upper arm length changed: {} -> {}",
            len_upper,
            new_len_upper
        );
        assert!(
            (len_forearm - new_len_forearm).abs() < 1e-4,
            "Forearm length changed: {} -> {}",
            len_forearm,
            new_len_forearm
        );
    }

    #[test]
    fn test_animation_interpolation() {
        let pose_a = RotationPose::bind_pose();
        let pose_b = RotationPose::bind_pose();

        // Rotate spine 90 degrees around X in pose B
        let pose_b = pose_b.with_rotation(
            BoneId::Spine,
            Quat::from_rotation_x(std::f32::consts::PI / 2.0),
        );

        let kf_a = RotationKeyframe {
            time: 0.0,
            pose: pose_a,
        };
        let kf_b = RotationKeyframe {
            time: 1.0,
            pose: pose_b,
        };

        let clip = RotationAnimationClip {
            name: "lerp_test".to_string(),
            duration: 1.0,
            keyframes: vec![kf_a, kf_b],
        };

        // Sample at 0.5
        let sample = clip.sample(0.5);

        let spine_rot = sample.local_rotations[BoneId::Spine.index()];
        let (axis, angle) = spine_rot.to_axis_angle();

        let expected_angle = std::f32::consts::PI / 4.0;

        if (axis - Vec3::X).length() < 0.01 {
            assert!(
                (angle - expected_angle).abs() < 0.01,
                "Angle should be 45 deg, got {}",
                angle.to_degrees()
            );
        }

        let rotated_y = spine_rot * Vec3::Y;
        assert!(
            (rotated_y.y - 0.707).abs() < 0.01,
            "Rotated Y.y should be ~0.707, got {}",
            rotated_y.y
        );
        assert!(
            (rotated_y.z - 0.707).abs() < 0.01,
            "Rotated Y.z should be ~0.707, got {}",
            rotated_y.z
        );
    }

    #[test]
    fn test_lazy_evaluation() {
        let pose = RotationPose::bind_pose();

        // Initially all dirty (inside private cache, hard to check directly via public API without exposing)
        // Check via cache (visible in child mod)
        assert!(pose.cache.borrow().dirty.is_dirty(BoneId::Head));

        // Access head position - should compute
        let _ = pose.get_position(BoneId::Head);

        // Now computed bones should be clean
        assert!(!pose.cache.borrow().dirty.is_dirty(BoneId::Hips));
        assert!(!pose.cache.borrow().dirty.is_dirty(BoneId::Spine));
        assert!(!pose.cache.borrow().dirty.is_dirty(BoneId::Head));
    }

    #[test]
    fn test_dirty_propagation() {
        let pose = RotationPose::bind_pose();
        pose.compute_all();

        // All clean now
        assert!(!pose.cache.borrow().dirty.is_dirty(BoneId::Head));

        // Rotate spine - should dirty head (child) but not legs
        let pose = pose.with_rotation(BoneId::Spine, Quat::from_rotation_x(0.5));

        assert!(pose.cache.borrow().dirty.is_dirty(BoneId::Spine));
        assert!(pose.cache.borrow().dirty.is_dirty(BoneId::Head)); // Child of spine
        assert!(!pose.cache.borrow().dirty.is_dirty(BoneId::LeftThigh)); // Not a child
    }

    #[test]
    fn test_euler_to_quat() {
        let euler = EulerAngles {
            x: 90.0,
            y: 0.0,
            z: 0.0,
        };
        let quat = euler.to_quat();

        let rotated = quat * Vec3::Y;
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
