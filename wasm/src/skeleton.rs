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
pub const BONE_RADIUS: f32 = 0.03;

/// Radius of the head sphere mesh
pub const HEAD_RADIUS: f32 = 0.10;

/// Radius of debug joint spheres (slightly larger than bones for visibility)
pub const JOINT_RADIUS: f32 = 0.04;

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
    /// Index into bone matrix array (0-22)
    pub bone_index: u32,
}

// Total number of renderable parts (bones)
// 21 cylinders (segments) + 1 head sphere = 22
// Just an estimate for buffer reservation, exact count not critical for constant but good for optimization
pub const RENDER_BONE_COUNT: usize = 22;

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
    add_cylinder_caps(vertices, start, end, radius, bone_idx, true, true);
}

fn add_cylinder_caps(
    vertices: &mut Vec<SkinnedVertex>,
    start: Vec3A,
    end: Vec3A,
    radius: f32,
    bone_idx: u32,
    start_cap: bool,
    end_cap: bool,
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

    // define basis for caps (same as cylinder body)
    // right and up are already defined. dir is the axis.

    // Helper for cap vertex generation
    let add_cap_ring = |vertices: &mut Vec<SkinnedVertex>,
                        center: Vec3A,
                        forward: Vec3A, // Normal direction of the pole (e.g. dir for end cap, -dir for start cap)
                        radius: f32| {
        let rings = SPHERE_LAT_SEGMENTS / 2; // Hemisphere
        let slices = CYLINDER_SEGMENTS;

        for r in 0..rings {
            let lat1 = (r as f32 / rings as f32) * (std::f32::consts::PI / 2.0);
            let lat2 = ((r + 1) as f32 / rings as f32) * (std::f32::consts::PI / 2.0);

            let y1 = lat1.sin(); // height along forward axis (0 to 1)
            let r1 = lat1.cos(); // radius at this height (1 to 0)
            let y2 = lat2.sin();
            let r2 = lat2.cos();

            for s in 0..slices {
                let lon1 = (s as f32 / slices as f32) * std::f32::consts::TAU;
                let lon2 = ((s + 1) as f32 / slices as f32) * std::f32::consts::TAU;

                // Function to compute vertex pos and normal
                // vertex = center + (right*cos(lon) + up*sin(lon))*r_scale*radius + forward*y_scale*radius
                let get_cap_vertex = |lat_r: f32, lat_y: f32, lon: f32| -> (Vec3A, Vec3A) {
                    let (sin_lon, cos_lon) = lon.sin_cos();
                    let radial = right * cos_lon + up * sin_lon; // vector in disk plane
                    let local_p = radial * (lat_r * radius) + forward * (lat_y * radius);
                    let normal = local_p.normalize();
                    (center + local_p, normal)
                };

                let (p1, n1) = get_cap_vertex(r1, y1, lon1);
                let (p2, n2) = get_cap_vertex(r1, y1, lon2);
                let (p3, n3) = get_cap_vertex(r2, y2, lon1);
                let (p4, n4) = get_cap_vertex(r2, y2, lon2);

                // Two triangles
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
        }
    };

    // Start Cap (Hemisphere pointing backwards)
    if start_cap {
        add_cap_ring(vertices, start, -dir, radius);
    }

    // End Cap (Hemisphere pointing forwards)
    if end_cap {
        add_cap_ring(vertices, start + dir * valid_len, dir, radius);
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

    // Matrix index - must be incremented for each primitive (cylinder/sphere)
    // This matches the order in pose.rs compute_bone_matrices()
    let mut idx: u32 = 0;

    // Spine chain (5 cylinders)
    add_cylinder(
        &mut vertices,
        DEFAULT_PELVIS,
        DEFAULT_SPINE1,
        BONE_RADIUS,
        idx,
    );
    idx += 1;
    add_cylinder(
        &mut vertices,
        DEFAULT_SPINE1,
        DEFAULT_SPINE2,
        BONE_RADIUS,
        idx,
    );
    idx += 1;
    add_cylinder(
        &mut vertices,
        DEFAULT_SPINE2,
        DEFAULT_SPINE3,
        BONE_RADIUS,
        idx,
    );
    idx += 1;
    add_cylinder(
        &mut vertices,
        DEFAULT_SPINE3,
        DEFAULT_NECK,
        BONE_RADIUS,
        idx,
    );
    idx += 1;
    // Neck -> Head
    // Shorten the neck so it connects to the base of the head sphere (surface)
    // instead of the center, to avoid visual artifacts inside the head.
    let neck_dir = (DEFAULT_HEAD - DEFAULT_NECK).normalize();
    // Stop slightly inside the head to ensure connection (0.8 * radius)
    let neck_end = DEFAULT_HEAD - neck_dir * (HEAD_RADIUS * 0.8);

    // Disable end cap (false) to avoid it protruding into the head sphere
    add_cylinder_caps(
        &mut vertices,
        DEFAULT_NECK,
        neck_end,
        BONE_RADIUS,
        idx,
        true,
        false,
    );
    idx += 1;

    // Left Arm chain (4 cylinders)
    add_cylinder(
        &mut vertices,
        DEFAULT_SPINE3,
        DEFAULT_LEFT_COLLAR,
        BONE_RADIUS,
        idx,
    );
    idx += 1;
    add_cylinder(
        &mut vertices,
        DEFAULT_LEFT_COLLAR,
        DEFAULT_LEFT_SHOULDER,
        BONE_RADIUS,
        idx,
    );
    idx += 1;
    add_cylinder(
        &mut vertices,
        DEFAULT_LEFT_SHOULDER,
        DEFAULT_LEFT_ELBOW,
        BONE_RADIUS,
        idx,
    );
    idx += 1;
    add_cylinder(
        &mut vertices,
        DEFAULT_LEFT_ELBOW,
        DEFAULT_LEFT_WRIST,
        BONE_RADIUS,
        idx,
    );
    idx += 1;

    // Right Arm chain (4 cylinders)
    add_cylinder(
        &mut vertices,
        DEFAULT_SPINE3,
        DEFAULT_RIGHT_COLLAR,
        BONE_RADIUS,
        idx,
    );
    idx += 1;
    add_cylinder(
        &mut vertices,
        DEFAULT_RIGHT_COLLAR,
        DEFAULT_RIGHT_SHOULDER,
        BONE_RADIUS,
        idx,
    );
    idx += 1;
    add_cylinder(
        &mut vertices,
        DEFAULT_RIGHT_SHOULDER,
        DEFAULT_RIGHT_ELBOW,
        BONE_RADIUS,
        idx,
    );
    idx += 1;
    add_cylinder(
        &mut vertices,
        DEFAULT_RIGHT_ELBOW,
        DEFAULT_RIGHT_WRIST,
        BONE_RADIUS,
        idx,
    );
    idx += 1;

    // Left Leg chain (4 cylinders)
    add_cylinder(
        &mut vertices,
        DEFAULT_PELVIS,
        DEFAULT_LEFT_HIP,
        BONE_RADIUS,
        idx,
    );
    idx += 1;
    add_cylinder(
        &mut vertices,
        DEFAULT_LEFT_HIP,
        DEFAULT_LEFT_KNEE,
        BONE_RADIUS,
        idx,
    );
    idx += 1;
    add_cylinder(
        &mut vertices,
        DEFAULT_LEFT_KNEE,
        DEFAULT_LEFT_ANKLE,
        BONE_RADIUS,
        idx,
    );
    idx += 1;
    add_cylinder(
        &mut vertices,
        DEFAULT_LEFT_ANKLE,
        DEFAULT_LEFT_FOOT,
        BONE_RADIUS,
        idx,
    );
    idx += 1;

    // Right Leg chain (4 cylinders)
    add_cylinder(
        &mut vertices,
        DEFAULT_PELVIS,
        DEFAULT_RIGHT_HIP,
        BONE_RADIUS,
        idx,
    );
    idx += 1;
    add_cylinder(
        &mut vertices,
        DEFAULT_RIGHT_HIP,
        DEFAULT_RIGHT_KNEE,
        BONE_RADIUS,
        idx,
    );
    idx += 1;
    add_cylinder(
        &mut vertices,
        DEFAULT_RIGHT_KNEE,
        DEFAULT_RIGHT_ANKLE,
        BONE_RADIUS,
        idx,
    );
    idx += 1;
    add_cylinder(
        &mut vertices,
        DEFAULT_RIGHT_ANKLE,
        DEFAULT_RIGHT_FOOT,
        BONE_RADIUS,
        idx,
    );
    idx += 1;

    // Head Sphere (1 sphere)
    add_sphere(&mut vertices, DEFAULT_HEAD, HEAD_RADIUS, idx);
    // idx += 1; // Last part, no need to increment

    // Debug joints (22 spheres) - REMOVED
    /*
    let all_defaults = [
        DEFAULT_PELVIS,
        DEFAULT_LEFT_HIP,
        DEFAULT_RIGHT_HIP,
        DEFAULT_SPINE1,
        DEFAULT_LEFT_KNEE,
        DEFAULT_RIGHT_KNEE,
        DEFAULT_SPINE2,
        DEFAULT_LEFT_ANKLE,
        DEFAULT_RIGHT_ANKLE,
        DEFAULT_SPINE3,
        DEFAULT_LEFT_FOOT,
        DEFAULT_RIGHT_FOOT,
        DEFAULT_NECK,
        DEFAULT_LEFT_COLLAR,
        DEFAULT_RIGHT_COLLAR,
        DEFAULT_HEAD,
        DEFAULT_LEFT_SHOULDER,
        DEFAULT_RIGHT_SHOULDER,
        DEFAULT_LEFT_ELBOW,
        DEFAULT_RIGHT_ELBOW,
        DEFAULT_LEFT_WRIST,
        DEFAULT_RIGHT_WRIST,
    ];

    for default_pos in all_defaults {
        add_sphere(&mut vertices, default_pos, JOINT_RADIUS, idx);
        idx += 1;
    }
    */

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_bind_pose_mesh_validity() {
        let vertices = generate_bind_pose_mesh();

        // Basic validity
        // Bounds check (roughly)
        // 21 cylinders + 1 head sphere + 22 joint spheres
        // Each cylinder (12 segments): (12*2 body + 12*2 caps) = 48 tris = 144 vertices
        // Spine chain (5) + Arms (4*2) + Legs (4*2) = 21 cylinders.
        // 21 * 144 = 3024 vertices.
        // Sphere (16 lat * 24 lon * 2 tris) = 768 tris = 2304 vertices.
        // Debug joints (22) = 22 * 2304 = 50688 vertices.
        // Total should be around 56k vertices.
        assert!(!vertices.is_empty(), "Mesh should not be empty");
        // Let's just check it's substantial.
        assert!(vertices.len() > 10_000);
        assert_eq!(
            vertices.len() % 3,
            0,
            "Vertex count must be multiple of 3 (triangles)"
        );

        // Check normals are normalized
        for v in &vertices {
            let n = Vec3::from_array(v.normal);
            let len = n.length();
            assert!(
                (len - 1.0).abs() < 0.01,
                "Normal length should be approx 1.0, got {}",
                len
            );

            // Bone index should be within reason
            assert!(v.bone_index < 50, "Bone index out of range");
        }
    }

    #[test]
    fn test_aligned_matrix() {
        let start = Vec3A::ZERO;
        let end = Vec3A::X; // aligned with X

        let target_start = Vec3A::new(10.0, 0.0, 0.0);
        let target_end = Vec3A::new(10.0, 1.0, 0.0); // Now pointing Y

        let mat = compute_aligned_matrix(start, end, target_start, target_end);

        // Transform the local X unit vector (the cylinder axis)
        let transformed_vector = mat.transform_vector3(Vec3::X);

        // It shoud roughly point +Y now
        assert!((transformed_vector.y - 1.0).abs() < 0.001);
        assert!(transformed_vector.x.abs() < 0.001);
    }
}
