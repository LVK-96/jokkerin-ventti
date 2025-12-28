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
