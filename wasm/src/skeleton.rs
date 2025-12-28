//! Skeleton system with glam-based bone positions
//!
//! CPU defines joint positions using glam::Vec3.
//! GPU generates cylinder/sphere geometry via instanced rendering.

use glam::{Vec3, Vec3A};

// Import shared skeleton constants for the default pose
use crate::skeleton_constants::*;

/// Radius constants for rendering
pub const BONE_RADIUS: f32 = 0.04;
pub const HEAD_RADIUS: f32 = 0.12;
pub const JOINT_RADIUS: f32 = 0.05; // Debug: slightly larger than bones so visible at exact positions

/// Vertex format for skinned mesh
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SkinnedVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub bone_index: u32,
}

// Total number of renderable parts (bones)
// 13 cylinders + 1 head sphere + 15 debug joint spheres = 29
pub const RENDER_BONE_COUNT: usize = 29;

fn add_cylinder(
    vertices: &mut Vec<SkinnedVertex>,
    start: Vec3A,
    end: Vec3A,
    radius: f32,
    bone_idx: u32,
) {
    let dir = (end - start).normalize();
    let length = start.distance(end);
    let valid_len = if length < 0.0001 { 0.0001 } else { length };

    let segments = 12;

    // Basis
    let up = if dir.abs().dot(Vec3A::Y) > 0.99 {
        Vec3A::X
    } else {
        Vec3A::Y
    };
    let right = dir.cross(up).normalize();
    let up = right.cross(dir).normalize();

    // Function to get point on ring at distance 'd' along bone
    let get_point = |angle: f32, d: f32, r: f32| -> (Vec3A, Vec3A) {
        let (sin, cos) = angle.sin_cos();
        let local_p = right * (cos * r) + up * (sin * r) + dir * d;
        let local_n = (right * cos + up * sin).normalize();
        (start + local_p, local_n)
    };

    // Body
    for i in 0..segments {
        let a1 = (i as f32 / segments as f32) * std::f32::consts::TAU;
        let a2 = ((i + 1) as f32 / segments as f32) * std::f32::consts::TAU;

        let (p1, n1) = get_point(a1, 0.0, radius);
        let (p2, n2) = get_point(a2, 0.0, radius);
        let (p3, n3) = get_point(a1, valid_len, radius);
        let (p4, n4) = get_point(a2, valid_len, radius);

        // Triangle 1
        vertices.push(SkinnedVertex {
            position: Vec3::from(p1).to_array(),
            normal: Vec3::from(n1).to_array(),
            bone_index: bone_idx,
        });
        vertices.push(SkinnedVertex {
            position: Vec3::from(p3).to_array(),
            normal: Vec3::from(n3).to_array(),
            bone_index: bone_idx,
        });
        vertices.push(SkinnedVertex {
            position: Vec3::from(p2).to_array(),
            normal: Vec3::from(n2).to_array(),
            bone_index: bone_idx,
        });

        // Triangle 2
        vertices.push(SkinnedVertex {
            position: Vec3::from(p2).to_array(),
            normal: Vec3::from(n2).to_array(),
            bone_index: bone_idx,
        });
        vertices.push(SkinnedVertex {
            position: Vec3::from(p3).to_array(),
            normal: Vec3::from(n3).to_array(),
            bone_index: bone_idx,
        });
        vertices.push(SkinnedVertex {
            position: Vec3::from(p4).to_array(),
            normal: Vec3::from(n4).to_array(),
            bone_index: bone_idx,
        });
    }

    // Caps (simple flat fan)
    // Start cap
    let center_start = start;
    let normal_start = -dir;
    for i in 0..segments {
        let a1 = (i as f32 / segments as f32) * std::f32::consts::TAU;
        let a2 = ((i + 1) as f32 / segments as f32) * std::f32::consts::TAU;
        let (p1, _) = get_point(a1, 0.0, radius);
        let (p2, _) = get_point(a2, 0.0, radius);

        vertices.push(SkinnedVertex {
            position: Vec3::from(center_start).to_array(),
            normal: Vec3::from(normal_start).to_array(),
            bone_index: bone_idx,
        });
        vertices.push(SkinnedVertex {
            position: Vec3::from(p2).to_array(),
            normal: Vec3::from(normal_start).to_array(),
            bone_index: bone_idx,
        });
        vertices.push(SkinnedVertex {
            position: Vec3::from(p1).to_array(),
            normal: Vec3::from(normal_start).to_array(),
            bone_index: bone_idx,
        });
    }

    // End cap
    let center_end = start + dir * valid_len;
    let normal_end = dir;
    for i in 0..segments {
        let a1 = (i as f32 / segments as f32) * std::f32::consts::TAU;
        let a2 = ((i + 1) as f32 / segments as f32) * std::f32::consts::TAU;
        let (p1, _) = get_point(a1, valid_len, radius);
        let (p2, _) = get_point(a2, valid_len, radius);

        vertices.push(SkinnedVertex {
            position: Vec3::from(center_end).to_array(),
            normal: Vec3::from(normal_end).to_array(),
            bone_index: bone_idx,
        });
        vertices.push(SkinnedVertex {
            position: Vec3::from(p1).to_array(),
            normal: Vec3::from(normal_end).to_array(),
            bone_index: bone_idx,
        });
        vertices.push(SkinnedVertex {
            position: Vec3::from(p2).to_array(),
            normal: Vec3::from(normal_end).to_array(),
            bone_index: bone_idx,
        });
    }
}

// Helper to add a sphere
fn add_sphere(vertices: &mut Vec<SkinnedVertex>, center: Vec3A, radius: f32, bone_idx: u32) {
    // Higher segment counts for smoother sphere silhouette
    let lat_segments = 16;
    let lon_segments = 24;

    for i in 0..lat_segments {
        let theta1 = (i as f32 / lat_segments as f32) * std::f32::consts::PI;
        let theta2 = ((i + 1) as f32 / lat_segments as f32) * std::f32::consts::PI;

        for j in 0..lon_segments {
            let phi1 = (j as f32 / lon_segments as f32) * std::f32::consts::TAU;
            let phi2 = ((j + 1) as f32 / lon_segments as f32) * std::f32::consts::TAU;

            let get_pos = |theta: f32, phi: f32| -> Vec3A {
                let sin_theta = theta.sin();
                Vec3A::new(
                    radius * sin_theta * phi.cos(),
                    radius * theta.cos(),
                    radius * sin_theta * phi.sin(),
                )
            };

            let p1 = get_pos(theta1, phi1);
            let p2 = get_pos(theta2, phi1);
            let p3 = get_pos(theta1, phi2);
            let p4 = get_pos(theta2, phi2);

            let n1 = p1.normalize();
            let n2 = p2.normalize();
            let n3 = p3.normalize();
            let n4 = p4.normalize();

            let w1 = center + p1;
            let w2 = center + p2;
            let w3 = center + p3;
            let w4 = center + p4;

            // Two triangles
            vertices.push(SkinnedVertex {
                position: Vec3::from(w1).to_array(),
                normal: Vec3::from(n1).to_array(),
                bone_index: bone_idx,
            });
            vertices.push(SkinnedVertex {
                position: Vec3::from(w2).to_array(),
                normal: Vec3::from(n2).to_array(),
                bone_index: bone_idx,
            });
            vertices.push(SkinnedVertex {
                position: Vec3::from(w3).to_array(),
                normal: Vec3::from(n3).to_array(),
                bone_index: bone_idx,
            });

            vertices.push(SkinnedVertex {
                position: Vec3::from(w2).to_array(),
                normal: Vec3::from(n2).to_array(),
                bone_index: bone_idx,
            });
            vertices.push(SkinnedVertex {
                position: Vec3::from(w4).to_array(),
                normal: Vec3::from(n4).to_array(),
                bone_index: bone_idx,
            });
            vertices.push(SkinnedVertex {
                position: Vec3::from(w3).to_array(),
                normal: Vec3::from(n3).to_array(),
                bone_index: bone_idx,
            });
        }
    }
}

pub fn generate_bind_pose_mesh() -> Vec<SkinnedVertex> {
    let mut vertices = Vec::new();
    let s = Skeleton::bind_pose();

    // Order MUST match compute_bone_matrices
    add_cylinder(&mut vertices, s.hips, s.neck, BONE_RADIUS, 0);
    add_cylinder(&mut vertices, s.neck, s.left_shoulder, BONE_RADIUS, 1);
    add_cylinder(&mut vertices, s.left_shoulder, s.left_elbow, BONE_RADIUS, 2);
    add_cylinder(&mut vertices, s.left_elbow, s.left_hand, BONE_RADIUS, 3);
    add_cylinder(&mut vertices, s.neck, s.right_shoulder, BONE_RADIUS, 4);
    add_cylinder(
        &mut vertices,
        s.right_shoulder,
        s.right_elbow,
        BONE_RADIUS,
        5,
    );
    add_cylinder(&mut vertices, s.right_elbow, s.right_hand, BONE_RADIUS, 6);
    add_cylinder(&mut vertices, s.hips, s.left_hip, BONE_RADIUS, 7);
    add_cylinder(&mut vertices, s.left_hip, s.left_knee, BONE_RADIUS, 8);
    add_cylinder(&mut vertices, s.left_knee, s.left_foot, BONE_RADIUS, 9);
    add_cylinder(&mut vertices, s.hips, s.right_hip, BONE_RADIUS, 10);
    add_cylinder(&mut vertices, s.right_hip, s.right_knee, BONE_RADIUS, 11);
    add_cylinder(&mut vertices, s.right_knee, s.right_foot, BONE_RADIUS, 12);

    add_sphere(&mut vertices, s.head, HEAD_RADIUS, 13);

    // Debug joints
    add_sphere(&mut vertices, s.hips, JOINT_RADIUS, 14);
    add_sphere(&mut vertices, s.neck, JOINT_RADIUS, 15);
    add_sphere(&mut vertices, s.left_shoulder, JOINT_RADIUS, 16);
    add_sphere(&mut vertices, s.left_elbow, JOINT_RADIUS, 17);
    add_sphere(&mut vertices, s.left_hand, JOINT_RADIUS, 18);
    add_sphere(&mut vertices, s.right_shoulder, JOINT_RADIUS, 19);
    add_sphere(&mut vertices, s.right_elbow, JOINT_RADIUS, 20);
    add_sphere(&mut vertices, s.right_hand, JOINT_RADIUS, 21);
    add_sphere(&mut vertices, s.left_hip, JOINT_RADIUS, 22);
    add_sphere(&mut vertices, s.left_knee, JOINT_RADIUS, 23);
    add_sphere(&mut vertices, s.left_foot, JOINT_RADIUS, 24);
    add_sphere(&mut vertices, s.right_hip, JOINT_RADIUS, 25);
    add_sphere(&mut vertices, s.right_knee, JOINT_RADIUS, 26);
    add_sphere(&mut vertices, s.right_foot, JOINT_RADIUS, 27);

    vertices
}

impl Skeleton {
    /// Compute all bone matrices for skinning
    /// Returns [Mat4; 29]
    pub fn compute_bone_matrices(&self) -> [glam::Mat4; RENDER_BONE_COUNT] {
        let bind = Skeleton::bind_pose();
        let mut matrices = [glam::Mat4::IDENTITY; RENDER_BONE_COUNT];

        // Cylinders
        matrices[0] = compute_aligned_matrix(bind.hips, bind.neck, self.hips, self.neck);
        matrices[1] =
            compute_aligned_matrix(bind.neck, bind.left_shoulder, self.neck, self.left_shoulder);
        matrices[2] = compute_aligned_matrix(
            bind.left_shoulder,
            bind.left_elbow,
            self.left_shoulder,
            self.left_elbow,
        );
        matrices[3] = compute_aligned_matrix(
            bind.left_elbow,
            bind.left_hand,
            self.left_elbow,
            self.left_hand,
        );
        matrices[4] = compute_aligned_matrix(
            bind.neck,
            bind.right_shoulder,
            self.neck,
            self.right_shoulder,
        );
        matrices[5] = compute_aligned_matrix(
            bind.right_shoulder,
            bind.right_elbow,
            self.right_shoulder,
            self.right_elbow,
        );
        matrices[6] = compute_aligned_matrix(
            bind.right_elbow,
            bind.right_hand,
            self.right_elbow,
            self.right_hand,
        );
        matrices[7] = compute_aligned_matrix(bind.hips, bind.left_hip, self.hips, self.left_hip);
        matrices[8] =
            compute_aligned_matrix(bind.left_hip, bind.left_knee, self.left_hip, self.left_knee);
        matrices[9] = compute_aligned_matrix(
            bind.left_knee,
            bind.left_foot,
            self.left_knee,
            self.left_foot,
        );
        matrices[10] = compute_aligned_matrix(bind.hips, bind.right_hip, self.hips, self.right_hip);
        matrices[11] = compute_aligned_matrix(
            bind.right_hip,
            bind.right_knee,
            self.right_hip,
            self.right_knee,
        );
        matrices[12] = compute_aligned_matrix(
            bind.right_knee,
            bind.right_foot,
            self.right_knee,
            self.right_foot,
        );

        // Head Sphere
        matrices[13] = compute_offset_matrix(bind.head, self.head);

        // Debug joints
        matrices[14] = compute_offset_matrix(bind.hips, self.hips);
        matrices[15] = compute_offset_matrix(bind.neck, self.neck);
        matrices[16] = compute_offset_matrix(bind.left_shoulder, self.left_shoulder);
        matrices[17] = compute_offset_matrix(bind.left_elbow, self.left_elbow);
        matrices[18] = compute_offset_matrix(bind.left_hand, self.left_hand);
        matrices[19] = compute_offset_matrix(bind.right_shoulder, self.right_shoulder);
        matrices[20] = compute_offset_matrix(bind.right_elbow, self.right_elbow);
        matrices[21] = compute_offset_matrix(bind.right_hand, self.right_hand);
        matrices[22] = compute_offset_matrix(bind.left_hip, self.left_hip);
        matrices[23] = compute_offset_matrix(bind.left_knee, self.left_knee);
        matrices[24] = compute_offset_matrix(bind.left_foot, self.left_foot);
        matrices[25] = compute_offset_matrix(bind.right_hip, self.right_hip);
        matrices[26] = compute_offset_matrix(bind.right_knee, self.right_knee);
        matrices[27] = compute_offset_matrix(bind.right_foot, self.right_foot);
        matrices[28] = compute_offset_matrix(bind.head, self.head);

        matrices
    }
}

fn compute_aligned_matrix(
    b_start: Vec3A,
    b_end: Vec3A,
    c_start: Vec3A,
    c_end: Vec3A,
) -> glam::Mat4 {
    let b_dir = (b_end - b_start).normalize();
    let c_dir = (c_end - c_start).normalize();
    let rot = glam::Quat::from_rotation_arc(Vec3::from(b_dir), Vec3::from(c_dir));
    glam::Mat4::from_translation(Vec3::from(c_start))
        * glam::Mat4::from_quat(rot)
        * glam::Mat4::from_translation(-Vec3::from(b_start))
}

fn compute_offset_matrix(b_center: Vec3A, c_center: Vec3A) -> glam::Mat4 {
    glam::Mat4::from_translation(Vec3::from(c_center - b_center))
}

/// Skeleton with named joint positions using glam::Vec3A
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Skeleton {
    pub hips: Vec3A,
    pub neck: Vec3A,
    pub head: Vec3A,
    pub left_shoulder: Vec3A,
    pub left_elbow: Vec3A,
    pub left_hand: Vec3A,
    pub right_shoulder: Vec3A,
    pub right_elbow: Vec3A,
    pub right_hand: Vec3A,
    pub left_hip: Vec3A,
    pub left_knee: Vec3A,
    pub left_foot: Vec3A,
    pub right_hip: Vec3A,
    pub right_knee: Vec3A,
    pub right_foot: Vec3A,
}

impl Default for Skeleton {
    fn default() -> Self {
        Self {
            hips: Vec3A::ZERO,
            neck: Vec3A::ZERO,
            head: Vec3A::ZERO,
            left_shoulder: Vec3A::ZERO,
            left_elbow: Vec3A::ZERO,
            left_hand: Vec3A::ZERO,
            right_shoulder: Vec3A::ZERO,
            right_elbow: Vec3A::ZERO,
            right_hand: Vec3A::ZERO,
            left_hip: Vec3A::ZERO,
            left_knee: Vec3A::ZERO,
            left_foot: Vec3A::ZERO,
            right_hip: Vec3A::ZERO,
            right_knee: Vec3A::ZERO,
            right_foot: Vec3A::ZERO,
        }
    }
}

impl Skeleton {
    /// The standard T-pose used as the base for all animations.
    pub fn bind_pose() -> Self {
        Self {
            hips: DEFAULT_HIPS,
            neck: DEFAULT_NECK,
            head: DEFAULT_HEAD,
            left_shoulder: DEFAULT_LEFT_SHOULDER,
            left_elbow: DEFAULT_LEFT_ELBOW,
            left_hand: DEFAULT_LEFT_HAND,
            right_shoulder: DEFAULT_RIGHT_SHOULDER,
            right_elbow: DEFAULT_RIGHT_ELBOW,
            right_hand: DEFAULT_RIGHT_HAND,
            left_hip: DEFAULT_LEFT_HIP,
            left_knee: DEFAULT_LEFT_KNEE,
            left_foot: DEFAULT_LEFT_FOOT,
            right_hip: DEFAULT_RIGHT_HIP,
            right_knee: DEFAULT_RIGHT_KNEE,
            right_foot: DEFAULT_RIGHT_FOOT,
        }
    }
}
