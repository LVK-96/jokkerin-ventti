use glam::Vec3;

/// Solve IK for a chain of joints using FABRIK algorithm
///
/// # Arguments
/// * `joints` - Current world positions of the joints in the chain (start to end effector)
/// * `lengths` - Lengths of the bones (distance between joint i and i+1)
/// * `target` - Target position for the end effector
///
/// # Returns
/// * `Vec<Vec3>` - New world positions for the joints
pub fn solve_fabrik(
    mut joints: Vec<Vec3>,
    lengths: &[f32],
    target: Vec3,
    max_iterations: usize,
    tolerance: f32,
) -> Vec<Vec3> {
    let n = joints.len();
    if n < 2 {
        return joints;
    }

    // Check reachability
    let dist = joints[0].distance(target);
    let total_len: f32 = lengths.iter().sum();

    // If unreachable, stretch straight towards target
    if dist > total_len {
        let dir = (target - joints[0]).normalize_or_zero();
        for i in 0..n - 1 {
            joints[i + 1] = joints[i] + dir * lengths[i];
        }
    } else {
        // Reachable - iterate
        let base_pos = joints[0];

        for _ in 0..max_iterations {
            if joints[n - 1].distance(target) < tolerance {
                break;
            }

            // Backward pass (end -> start)
            joints[n - 1] = target;
            for i in (0..n - 1).rev() {
                let dir = (joints[i] - joints[i + 1]).normalize_or_zero();
                joints[i] = joints[i + 1] + dir * lengths[i];
            }

            // Forward pass (start -> end)
            joints[0] = base_pos;
            for i in 0..n - 1 {
                let dir = (joints[i + 1] - joints[i]).normalize_or_zero();
                joints[i + 1] = joints[i] + dir * lengths[i];
            }
        }
    }
    joints
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn test_fabrik_reachable_target() {
        // 2-joint chain (1 bone) reaching a target exactly at distance length
        let joints = vec![Vec3::ZERO, Vec3::new(2.0, 0.0, 0.0)];
        let lengths = vec![2.0];
        let target = Vec3::new(0.0, 2.0, 0.0); // Distance 2.0 from base

        let result = solve_fabrik(joints, &lengths, target, 10, 0.001);

        assert!(result[0].distance(Vec3::ZERO) < 0.001);
        assert!(result[1].distance(target) < 0.001);
        // Check length preserved
        assert!((result[0].distance(result[1]) - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_fabrik_unreachable_target() {
        // Target beyond max reach
        let joints = vec![
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
        ];
        let lengths = vec![1.0, 1.0]; // Max reach 2.0
        let target = Vec3::new(3.0, 0.0, 0.0);

        let result = solve_fabrik(joints, &lengths, target, 10, 0.001);

        assert!(result[0].distance(Vec3::ZERO) < 0.001);
        // Should stretch towards target
        // Direction is (1,0,0)
        assert!(result[1].distance(Vec3::new(1.0, 0.0, 0.0)) < 0.001);
        assert!(result[2].distance(Vec3::new(2.0, 0.0, 0.0)) < 0.001);
    }

    #[test]
    fn test_fabrik_multi_joint_chain() {
        // 3-joint arm (2 bones) reaching target
        let joints = vec![
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
        ];
        let lengths = vec![1.0, 1.0];
        let target = Vec3::new(1.0, 1.0, 0.0); // Reachable (dist sqrt(2) approx 1.41)

        let result = solve_fabrik(joints, &lengths, target, 20, 0.001);

        assert!(result[0].distance(Vec3::ZERO) < 0.001);
        assert!(result[2].distance(target) < 0.01); // Tolerance

        // Check lengths preserved
        assert!((result[0].distance(result[1]) - 1.0).abs() < 0.001);
        assert!((result[1].distance(result[2]) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_fabrik_single_joint() {
        // Edge case with 1 joint
        let joints = vec![Vec3::ZERO];
        let lengths = vec![];
        let target = Vec3::new(1.0, 0.0, 0.0);

        let result = solve_fabrik(joints, &lengths, target, 10, 0.001);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0], Vec3::ZERO);
    }

    #[test]
    fn test_fabrik_preserves_base() {
        let joints = vec![Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0)];
        let lengths = vec![1.0];
        let target = Vec3::new(0.5, 0.5, 0.0);

        let result = solve_fabrik(joints, &lengths, target, 10, 0.001);

        assert!(result[0].distance(Vec3::ZERO) < 0.001);
    }
}
