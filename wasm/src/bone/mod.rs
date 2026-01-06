pub mod anim_ids;
pub mod cache;
pub mod clip;
pub mod id;
pub mod pose;

pub use anim_ids::*;
pub use cache::*;
pub use clip::*;
pub use id::*;
pub use pose::*;

#[cfg(test)]
mod tests {
    use super::*;

    use glam::{Quat, Vec3};
    use wasm_bindgen_test::*;

    #[test]
    #[wasm_bindgen_test]
    fn test_bind_pose_positions() {
        let pose = RotationPose::bind_pose();
        // Force computation to ensure world positions are ready
        pose.compute_all();

        let pelvis_pos = pose.get_position(BoneId::Pelvis);

        // Pelvis should be at DEFAULT_HIPS_Y (assuming it's defined or we check relative)
        // Note: DEFAULT_HIPS_Y might need to be renamed too but let's see if it compiles

        // Head should be above pelvis
        let head_pos = pose.get_position(BoneId::Head);
        assert!(head_pos.y > pelvis_pos.y);

        // Feet should be near ground
        let left_foot_pos = pose.get_position(BoneId::LeftAnkle);
        let right_foot_pos = pose.get_position(BoneId::RightAnkle);

        assert!(left_foot_pos.y < 0.2);
        assert!(right_foot_pos.y < 0.2);
    }

    #[test]
    #[wasm_bindgen_test]
    fn test_floor_constraint() {
        let pose = RotationPose::bind_pose();
        // Move root way below floor
        let mut pose = pose.with_root_position(Vec3::new(0.0, -2.0, 0.0));
        pose.compute_all();

        // Verify it's below floor
        assert!(pose.get_position(BoneId::Pelvis).y < 0.0);

        // Apply constraint
        pose = pose.apply_floor_constraint();

        // Should be lifted
        let pelvis_y = pose.get_position(BoneId::Pelvis).y;
        assert!(
            pelvis_y > 0.0,
            "Pelvis should be above floor, got {}",
            pelvis_y
        );

        // More precise check: calculate min y of all bones
        let mut min_y = f32::MAX;
        for bone in BoneId::ALL {
            min_y = min_y.min(pose.get_position(bone).y);
        }
        assert!(
            min_y >= -0.1,
            "Lowest bone should be near or above 0, got {}",
            min_y
        );
    }

    #[test]
    #[wasm_bindgen_test]
    fn test_ik_preserves_chain_lengths() {
        let pose = RotationPose::bind_pose();
        let chain = [BoneId::LeftCollar, BoneId::LeftShoulder, BoneId::LeftElbow];

        // Get initial lengths
        let pos_collar = pose.get_position(BoneId::LeftCollar);
        let pos_shoulder = pose.get_position(BoneId::LeftShoulder);
        let pos_elbow = pose.get_position(BoneId::LeftElbow);

        let len_upper = pos_collar.distance(pos_shoulder);
        let len_forearm = pos_shoulder.distance(pos_elbow);

        // Apply IK to a new target
        let target = Vec3::new(0.5, 0.5, 0.5);
        let pose = pose.apply_ik(&chain, target);

        let new_pos_collar = pose.get_position(BoneId::LeftCollar);
        let new_pos_shoulder = pose.get_position(BoneId::LeftShoulder);
        let new_pos_elbow = pose.get_position(BoneId::LeftElbow);

        let new_len_upper = new_pos_collar.distance(new_pos_shoulder);
        let new_len_forearm = new_pos_shoulder.distance(new_pos_elbow);

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
    #[wasm_bindgen_test]
    fn test_animation_interpolation() {
        let pose_a = RotationPose::bind_pose();
        let pose_b = RotationPose::bind_pose();

        // Rotate spine 90 degrees around X in pose B
        let pose_b = pose_b.with_rotation(
            BoneId::Spine1,
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

        let spine_rot = sample.local_rotations[BoneId::Spine1.index()];
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
            (rotated_y.y - 0.707).abs() < 0.1,
            "Rotated Y.y should be ~0.707, got {}",
            rotated_y.y
        );
        assert!(
            (rotated_y.z - 0.707).abs() < 0.1,
            "Rotated Y.z should be ~0.707, got {}",
            rotated_y.z
        );
    }

    #[test]
    #[wasm_bindgen_test]
    fn test_lazy_evaluation() {
        let pose = RotationPose::bind_pose();

        // Initially all dirty
        assert!(pose.cache.borrow().dirty.is_dirty(BoneId::Head));

        // Access head position - should compute
        let _ = pose.get_position(BoneId::Head);

        // Now computed bones should be clean
        assert!(!pose.cache.borrow().dirty.is_dirty(BoneId::Pelvis));
        assert!(!pose.cache.borrow().dirty.is_dirty(BoneId::Spine1));
        assert!(!pose.cache.borrow().dirty.is_dirty(BoneId::Head));
    }

    #[test]
    #[wasm_bindgen_test]
    fn test_dirty_propagation() {
        let pose = RotationPose::bind_pose();
        pose.compute_all();

        // All clean now
        assert!(!pose.cache.borrow().dirty.is_dirty(BoneId::Head));

        // Rotate spine - should dirty head (child) but not legs
        let pose = pose.with_rotation(BoneId::Spine1, Quat::from_rotation_x(0.5));

        assert!(pose.cache.borrow().dirty.is_dirty(BoneId::Spine1));
        assert!(pose.cache.borrow().dirty.is_dirty(BoneId::Head)); // Child of spine
        assert!(!pose.cache.borrow().dirty.is_dirty(BoneId::LeftHip)); // Not a child
    }

    #[test]
    #[wasm_bindgen_test]
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
    #[wasm_bindgen_test]
    fn test_animation_parsing() {
        // Uses v2 schema with short field names
        let json = r#"{
            "v": 2,
            "n": "test",
            "d": 1.0,
            "kf": [
                {
                    "t": 0.0,
                    "p": {
                        "s1": { "x": 0, "y": 0, "z": 0 }
                    }
                },
                {
                    "t": 0.5,
                    "p": {
                        "s1": { "w": 1.0, "x": 0.0, "y": 0.0, "z": 0.0 }
                    }
                }
            ]
        }"#;

        let clip = RotationAnimationClip::from_json(json).unwrap();
        assert_eq!(clip.name, "test");
        assert_eq!(clip.keyframes.len(), 2);
    }

    #[test]
    #[wasm_bindgen_test]
    fn test_binary_animation_parsing() {
        let mut data = Vec::new();

        // 1. Basic Header (8 bytes)
        data.extend_from_slice(&2u16.to_le_bytes()); // 2 keyframes
        data.extend_from_slice(&[0x00, 0x45]); // f16 for 5.0
        data.extend_from_slice(&0u32.to_le_bytes()); // dynamic_mask = 0 (static)

        // 2. Base Data (Header extension)
        // Base Root (0, 1.0, 0)
        data.extend_from_slice(&[0x00, 0x00]); // x = 0
        data.extend_from_slice(&[0x00, 0x3c]); // y = 1.0
        data.extend_from_slice(&[0x00, 0x00]); // z = 0

        for _ in 0..22 {
            data.extend_from_slice(&0i16.to_le_bytes()); // x = 0
            data.extend_from_slice(&0i16.to_le_bytes()); // y = 0
            data.extend_from_slice(&0i16.to_le_bytes()); // z = 0
        }

        let clip = RotationAnimationClip::from_binary(&data, "test".to_string()).unwrap();
        assert_eq!(clip.keyframes.len(), 2);
        assert!((clip.duration - 5.0).abs() < 0.1);
        assert!((clip.keyframes[0].pose.root_position.y - 1.0).abs() < 0.01);
    }
}
