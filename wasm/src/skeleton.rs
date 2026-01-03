//! Skeleton system with glam-based bone positions
//!
//! CPU defines joint positions using glam::Vec3.
//! GPU generates cylinder/sphere geometry via instanced rendering.

use glam::{Vec3, Vec3A};

/// Radius constants for rendering and physics
///
/// BONE_RADIUS is used for:
/// - Cylinder geometry thickness when rendering bones
/// - Floor collision detection (joints must be above BONE_RADIUS to avoid clipping)
pub const BONE_RADIUS: f32 = 0.04;

/// Radius of the head sphere mesh
pub const HEAD_RADIUS: f32 = 0.12;

/// Radius of debug joint spheres (slightly larger than bones for visibility)
pub const JOINT_RADIUS: f32 = 0.05;

/// Vertex format for skinned mesh
/// Vertex format for GPU-skinned mesh rendering
///
/// Each vertex is transformed by a bone matrix indexed by `bone_index`.
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SkinnedVertex {
    /// Local position relative to bone origin
    pub position: [f32; 3],
    /// Surface normal for lighting
    pub normal: [f32; 3],
    /// Index into bone matrix array (0-28)
    pub bone_index: u32,
}

// Total number of renderable parts (bones)
// 13 cylinders + 1 head sphere + 15 debug joint spheres = 29
pub const RENDER_BONE_COUNT: usize = 29;

/// Number of segments for cylinder geometry
pub const CYLINDER_SEGMENTS: usize = 12;
/// Number of latitude segments for sphere geometry
pub const SPHERE_LAT_SEGMENTS: usize = 16;
/// Number of longitude segments for sphere geometry
pub const SPHERE_LON_SEGMENTS: usize = 24;

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

    let segments = CYLINDER_SEGMENTS;

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
    let lat_segments = SPHERE_LAT_SEGMENTS;
    let lon_segments = SPHERE_LON_SEGMENTS;

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
    use crate::skeleton_constants::*;

    // Order MUST match compute_bone_matrices
    add_cylinder(&mut vertices, DEFAULT_HIPS, DEFAULT_NECK, BONE_RADIUS, 0);
    add_cylinder(
        &mut vertices,
        DEFAULT_NECK,
        DEFAULT_LEFT_SHOULDER,
        BONE_RADIUS,
        1,
    );
    add_cylinder(
        &mut vertices,
        DEFAULT_LEFT_SHOULDER,
        DEFAULT_LEFT_ELBOW,
        BONE_RADIUS,
        2,
    );
    add_cylinder(
        &mut vertices,
        DEFAULT_LEFT_ELBOW,
        DEFAULT_LEFT_HAND,
        BONE_RADIUS,
        3,
    );
    add_cylinder(
        &mut vertices,
        DEFAULT_NECK,
        DEFAULT_RIGHT_SHOULDER,
        BONE_RADIUS,
        4,
    );
    add_cylinder(
        &mut vertices,
        DEFAULT_RIGHT_SHOULDER,
        DEFAULT_RIGHT_ELBOW,
        BONE_RADIUS,
        5,
    );
    add_cylinder(
        &mut vertices,
        DEFAULT_RIGHT_ELBOW,
        DEFAULT_RIGHT_HAND,
        BONE_RADIUS,
        6,
    );
    add_cylinder(
        &mut vertices,
        DEFAULT_HIPS,
        DEFAULT_LEFT_HIP,
        BONE_RADIUS,
        7,
    );
    add_cylinder(
        &mut vertices,
        DEFAULT_LEFT_HIP,
        DEFAULT_LEFT_KNEE,
        BONE_RADIUS,
        8,
    );
    add_cylinder(
        &mut vertices,
        DEFAULT_LEFT_KNEE,
        DEFAULT_LEFT_FOOT,
        BONE_RADIUS,
        9,
    );
    add_cylinder(
        &mut vertices,
        DEFAULT_HIPS,
        DEFAULT_RIGHT_HIP,
        BONE_RADIUS,
        10,
    );
    add_cylinder(
        &mut vertices,
        DEFAULT_RIGHT_HIP,
        DEFAULT_RIGHT_KNEE,
        BONE_RADIUS,
        11,
    );
    add_cylinder(
        &mut vertices,
        DEFAULT_RIGHT_KNEE,
        DEFAULT_RIGHT_FOOT,
        BONE_RADIUS,
        12,
    );

    add_sphere(&mut vertices, DEFAULT_HEAD, HEAD_RADIUS, 13);

    // Debug joints
    add_sphere(&mut vertices, DEFAULT_HIPS, JOINT_RADIUS, 14);
    add_sphere(&mut vertices, DEFAULT_NECK, JOINT_RADIUS, 15);
    add_sphere(&mut vertices, DEFAULT_LEFT_SHOULDER, JOINT_RADIUS, 16);
    add_sphere(&mut vertices, DEFAULT_LEFT_ELBOW, JOINT_RADIUS, 17);
    add_sphere(&mut vertices, DEFAULT_LEFT_HAND, JOINT_RADIUS, 18);
    add_sphere(&mut vertices, DEFAULT_RIGHT_SHOULDER, JOINT_RADIUS, 19);
    add_sphere(&mut vertices, DEFAULT_RIGHT_ELBOW, JOINT_RADIUS, 20);
    add_sphere(&mut vertices, DEFAULT_RIGHT_HAND, JOINT_RADIUS, 21);
    add_sphere(&mut vertices, DEFAULT_LEFT_HIP, JOINT_RADIUS, 22);
    add_sphere(&mut vertices, DEFAULT_LEFT_KNEE, JOINT_RADIUS, 23);
    add_sphere(&mut vertices, DEFAULT_LEFT_FOOT, JOINT_RADIUS, 24);
    add_sphere(&mut vertices, DEFAULT_RIGHT_HIP, JOINT_RADIUS, 25);
    add_sphere(&mut vertices, DEFAULT_RIGHT_KNEE, JOINT_RADIUS, 26);
    add_sphere(&mut vertices, DEFAULT_RIGHT_FOOT, JOINT_RADIUS, 27);

    vertices
}

pub fn compute_aligned_matrix(
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

pub fn compute_offset_matrix(b_center: Vec3A, c_center: Vec3A) -> glam::Mat4 {
    glam::Mat4::from_translation(Vec3::from(c_center - b_center))
}
