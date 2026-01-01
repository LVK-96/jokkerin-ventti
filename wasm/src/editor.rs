use crate::bone::BoneId;
use wasm_bindgen::prelude::*;

/// Helper to access the current session with mutable access
fn with_session_mut<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut crate::state::EditorSession) -> R,
{
    crate::state::with_app_state_mut(|app| app.editor_mut().map(f)).flatten()
}

/// Helper to access the current session with read-only access
fn with_session<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&crate::state::EditorSession) -> R,
{
    crate::state::with_app_state(|app| app.editor().map(f)).flatten()
}

/// Start editing an animation clip for the given exercise
#[wasm_bindgen]
pub fn start_editing(exercise_name: &str) -> bool {
    let name = exercise_name.to_string();

    crate::state::with_app_state_mut(|app| {
        if let Some(clip) = app.animation_library.get_clip(&name).cloned() {
            app.start_editing(clip);
            log::info!("Started editing: {}", name);
            true
        } else {
            log::warn!("No animation loaded for exercise: {}", name);
            false
        }
    })
    .unwrap_or(false)
}

/// Stop editing and clear the current session
#[wasm_bindgen]
pub fn stop_editing() {
    crate::state::with_app_state_mut(|app| {
        app.stop_editing();
        log::info!("Stopped editing");
    });
}

/// Check if an editing session is active
#[wasm_bindgen]
pub fn is_editing() -> bool {
    with_session(|_| true).unwrap_or(false)
}

fn get_bone_and_chain(joint_index: usize) -> Option<(BoneId, Vec<BoneId>)> {
    match joint_index {
        // IK Chains (End Effectors)
        6 => Some((
            BoneId::LeftForearm,
            vec![
                BoneId::LeftShoulder,
                BoneId::LeftUpperArm,
                BoneId::LeftForearm,
            ],
        )),
        9 => Some((
            BoneId::RightForearm,
            vec![
                BoneId::RightShoulder,
                BoneId::RightUpperArm,
                BoneId::RightForearm,
            ],
        )),
        12 => Some((BoneId::LeftShin, vec![BoneId::LeftThigh, BoneId::LeftShin])),
        15 => Some((
            BoneId::RightShin,
            vec![BoneId::RightThigh, BoneId::RightShin],
        )),
        3 => Some((
            BoneId::Head,
            vec![BoneId::Spine, BoneId::Neck, BoneId::Head],
        )),
        1 => Some((BoneId::Spine, vec![BoneId::Spine])),
        2 => Some((BoneId::Neck, vec![BoneId::Neck])),

        // FK Bones (Intermediate Joints)
        5 => Some((BoneId::LeftUpperArm, vec![])),
        8 => Some((BoneId::RightUpperArm, vec![])),
        4 => Some((BoneId::LeftShoulder, vec![])),
        7 => Some((BoneId::RightShoulder, vec![])),
        11 => Some((BoneId::LeftThigh, vec![])),
        14 => Some((BoneId::RightThigh, vec![])),

        _ => None,
    }
}

/// Get the number of keyframes in the current clip
#[wasm_bindgen]
pub fn get_keyframe_count() -> usize {
    with_session(|session| session.clip.keyframes.len()).unwrap_or(0)
}

/// Get the time of the current keyframe
#[wasm_bindgen]
pub fn get_keyframe_time() -> f32 {
    with_session(|session| {
        session
            .clip
            .keyframes
            .get(session.keyframe_index)
            .map(|kf| kf.time)
            .unwrap_or(0.0)
    })
    .unwrap_or(0.0)
}

/// Get the current keyframe index
#[wasm_bindgen]
pub fn get_keyframe_index() -> usize {
    with_session(|session| session.keyframe_index).unwrap_or(0)
}

/// Set the current keyframe index for editing
#[wasm_bindgen]
pub fn set_keyframe_index(index: usize) {
    with_session_mut(|session| {
        if index < session.clip.keyframes.len() {
            session.keyframe_index = index;
        }
    });
}

/// Add a new keyframe as a copy of the one at after_index
#[wasm_bindgen]
pub fn add_keyframe(after_index: usize) {
    with_session_mut(|session| {
        let clip = &mut session.clip;
        if after_index < clip.keyframes.len() {
            let prev_keyframe = clip.keyframes[after_index].clone();
            let new_time = if after_index + 1 < clip.keyframes.len() {
                (prev_keyframe.time + clip.keyframes[after_index + 1].time) / 2.0
            } else {
                prev_keyframe.time + 0.5
            };

            let mut new_keyframe = prev_keyframe;
            new_keyframe.time = new_time;
            clip.keyframes.insert(after_index + 1, new_keyframe);

            if let Some(last) = clip.keyframes.last() {
                if last.time > clip.duration {
                    clip.duration = last.time + 0.5;
                }
            }
            log::info!("Added keyframe at time {:.2}s", new_time);
        }
    });
}

/// Remove a keyframe by index (won't remove last keyframe)
#[wasm_bindgen]
pub fn delete_keyframe(index: usize) {
    with_session_mut(|session| {
        let clip = &mut session.clip;
        if clip.keyframes.len() > 1 && index < clip.keyframes.len() {
            clip.keyframes.remove(index);
            // Adjust keyframe_index if needed
            if session.keyframe_index >= clip.keyframes.len() {
                session.keyframe_index = clip.keyframes.len().saturating_sub(1);
            }
            log::info!("Removed keyframe {}", index);
        }
    });
}

/// Export the current clip as JSON
#[wasm_bindgen]
pub fn export_clip_json() -> String {
    with_session(|session| {
        session.clip.to_json_string().unwrap_or_else(|e| {
            log::error!("Failed to export animation: {}", e);
            String::from("{}")
        })
    })
    .unwrap_or_else(|| String::from("{}"))
}

/// Joint position and rotation info for UI display
#[wasm_bindgen]
pub struct JointInfo {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub rx: f32,
    pub ry: f32,
    pub rz: f32,
}

/// Get joint info for a bone in the current keyframe
#[wasm_bindgen]
pub fn get_bone_info(bone_index: usize) -> Option<JointInfo> {
    if bone_index >= crate::bone::BoneId::COUNT {
        return None;
    }

    with_session(|session| {
        let pose = &session.clip.keyframes.get(session.keyframe_index)?.pose;
        let id = crate::bone::BoneId::ALL[bone_index];
        let rot = pose.local_rotations[bone_index];
        let (rx, ry, rz) = rot.to_euler(glam::EulerRot::XYZ);
        let pos = pose.get_position(id);

        Some(JointInfo {
            x: pos.x,
            y: pos.y,
            z: pos.z,
            rx: rx.to_degrees(),
            ry: ry.to_degrees(),
            rz: rz.to_degrees(),
        })
    })?
}

/// Set joint rotation for a bone in the current keyframe
#[wasm_bindgen]
pub fn set_bone_rotation(bone_index: usize, rx: f32, ry: f32, rz: f32) {
    if bone_index >= crate::bone::BoneId::COUNT {
        return;
    }

    with_session_mut(|session| {
        if session.keyframe_index < session.clip.keyframes.len() {
            let pose = &mut session.clip.keyframes[session.keyframe_index].pose;
            let id = crate::bone::BoneId::ALL[bone_index];
            let q = glam::Quat::from_euler(
                glam::EulerRot::XYZ,
                rx.to_radians(),
                ry.to_radians(),
                rz.to_radians(),
            );
            *pose =
                std::mem::replace(pose, crate::bone::RotationPose::default()).with_rotation(id, q);
        }
    });
}

/// Set joint position for a bone using IK/FK in the current keyframe
#[wasm_bindgen]
pub fn set_bone_position(bone_index: usize, x: f32, y: f32, z: f32) {
    with_session_mut(|session| {
        if session.keyframe_index >= session.clip.keyframes.len() {
            return;
        }

        let pose = &mut session.clip.keyframes[session.keyframe_index].pose;
        let target_pos = glam::Vec3::new(x, y, z);

        if bone_index == 0 {
            // Root position
            *pose = std::mem::replace(pose, crate::bone::RotationPose::default())
                .with_root_position(target_pos)
                .apply_floor_constraint();
            return;
        }

        let (bone_id, chain) = match get_bone_and_chain(bone_index) {
            Some(res) => res,
            None => return,
        };

        if !chain.is_empty() {
            // IK
            *pose = std::mem::replace(pose, crate::bone::RotationPose::default())
                .apply_ik(&chain, target_pos)
                .apply_floor_constraint();
        } else {
            // FK Logic
            let pivot_pos =
                if let Some(parent) = crate::bone::BONE_HIERARCHY[bone_id.index()].parent {
                    pose.get_position(parent)
                } else {
                    pose.root_position
                };

            let target_dir = (target_pos - pivot_pos).normalize_or_zero();
            if target_dir.length_squared() > 1e-6 {
                let parent_rot = compute_world_rotation(pose, bone_id);
                let default_dir = crate::bone::BONE_HIERARCHY[bone_id.index()]
                    .direction
                    .normalize();
                let target_dir_local = parent_rot.inverse() * target_dir;
                let delta_rot = glam::Quat::from_rotation_arc(default_dir, target_dir_local);

                *pose = std::mem::replace(pose, crate::bone::RotationPose::default())
                    .with_rotation(bone_id, delta_rot.normalize());
            }

            // Apply floor constraint
            *pose = std::mem::replace(pose, crate::bone::RotationPose::default())
                .apply_floor_constraint();
        }
    });
}

/// Compute world rotation for a bone's parent (helper for FK)
fn compute_world_rotation(pose: &crate::bone::RotationPose, bone_id: BoneId) -> glam::Quat {
    if let Some(parent) = crate::bone::BONE_HIERARCHY[bone_id.index()].parent {
        let mut rot = pose.local_rotations[parent.index()];
        let mut p = parent;
        while let Some(grandparent) = crate::bone::BONE_HIERARCHY[p.index()].parent {
            rot = pose.local_rotations[grandparent.index()] * rot;
            p = grandparent;
        }
        rot
    } else {
        glam::Quat::IDENTITY
    }
}

/// Apply a screen-space drag to a joint
#[wasm_bindgen]
pub fn drag_joint(
    joint_index: usize,
    dx: f32,
    dy: f32,
    view: &[f32],
    proj: &[f32],
    width: f32,
    height: f32,
) {
    if view.len() < 16 || proj.len() < 16 {
        return;
    }

    let view_mat = glam::Mat4::from_cols_array(view.try_into().unwrap());
    let proj_mat = glam::Mat4::from_cols_array(proj.try_into().unwrap());

    with_session_mut(|session| {
        if session.keyframe_index >= session.clip.keyframes.len() {
            return;
        }

        let pose = &mut session.clip.keyframes[session.keyframe_index].pose;

        // Root drag
        if joint_index == 0 {
            let current_pos = pose.root_position;
            let target_pos =
                project_and_offset(current_pos, dx, dy, width, height, view_mat, proj_mat);
            *pose = std::mem::replace(pose, crate::bone::RotationPose::default())
                .with_root_position(target_pos)
                .apply_floor_constraint();
            return;
        }

        let (bone_id, chain) = match get_bone_and_chain(joint_index) {
            Some(res) => res,
            None => return,
        };

        let current_pos = pose.get_position(bone_id);
        let target_pos = project_and_offset(current_pos, dx, dy, width, height, view_mat, proj_mat);

        if !chain.is_empty() {
            // IK
            *pose = std::mem::replace(pose, crate::bone::RotationPose::default())
                .apply_ik(&chain, target_pos)
                .apply_floor_constraint();
        } else {
            // FK
            let pivot_pos =
                if let Some(parent) = crate::bone::BONE_HIERARCHY[bone_id.index()].parent {
                    pose.get_position(parent)
                } else {
                    pose.root_position
                };

            let target_dir = (target_pos - pivot_pos).normalize_or_zero();
            if target_dir.length_squared() > 1e-6 {
                let parent_rot = compute_world_rotation(pose, bone_id);
                let default_dir = crate::bone::BONE_HIERARCHY[bone_id.index()]
                    .direction
                    .normalize();
                let target_dir_local = parent_rot.inverse() * target_dir;
                let delta_rot = glam::Quat::from_rotation_arc(default_dir, target_dir_local);

                *pose = std::mem::replace(pose, crate::bone::RotationPose::default())
                    .with_rotation(bone_id, delta_rot.normalize());
            }
        }
    });
}

/// Get screen positions of all joints (for picking)
#[wasm_bindgen]
pub fn get_joint_positions(view: &[f32], proj: &[f32], width: f32, height: f32) -> Vec<f32> {
    if view.len() < 16 || proj.len() < 16 {
        return Vec::new();
    }

    let view_mat = glam::Mat4::from_cols_array(view.try_into().unwrap());
    let proj_mat = glam::Mat4::from_cols_array(proj.try_into().unwrap());
    let view_proj = proj_mat * view_mat;

    with_session(|session| {
        let pose = &session.clip.keyframes.get(session.keyframe_index)?.pose;
        pose.compute_all();
        let cache = pose.cache.borrow();
        use crate::bone::BoneId;
        use glam::{Vec3, Vec3A};

        let hips = Vec3A::from(pose.root_position);
        let left_hip_offset =
            cache.world_rotations[BoneId::Hips.index()] * Vec3::new(-0.02, -0.05, 0.0);
        let right_hip_offset =
            cache.world_rotations[BoneId::Hips.index()] * Vec3::new(0.02, -0.05, 0.0);

        let joints = [
            hips,                                                              // hips
            Vec3A::from(cache.world_positions[BoneId::Spine.index()]),         // neck
            Vec3A::from(cache.world_positions[BoneId::Spine.index()]), // neck (duplicate for some reason)
            Vec3A::from(cache.world_positions[BoneId::Head.index()]),  // head
            Vec3A::from(cache.world_positions[BoneId::LeftShoulder.index()]), // left_shoulder
            Vec3A::from(cache.world_positions[BoneId::LeftUpperArm.index()]), // left_elbow
            Vec3A::from(cache.world_positions[BoneId::LeftForearm.index()]), // left_hand
            Vec3A::from(cache.world_positions[BoneId::RightShoulder.index()]), // right_shoulder
            Vec3A::from(cache.world_positions[BoneId::RightUpperArm.index()]), // right_elbow
            Vec3A::from(cache.world_positions[BoneId::RightForearm.index()]), // right_hand
            Vec3A::from(pose.root_position + left_hip_offset),         // left_hip
            Vec3A::from(cache.world_positions[BoneId::LeftThigh.index()]), // left_knee
            Vec3A::from(cache.world_positions[BoneId::LeftShin.index()]), // left_foot
            Vec3A::from(pose.root_position + right_hip_offset),        // right_hip
            Vec3A::from(cache.world_positions[BoneId::RightThigh.index()]), // right_knee
            Vec3A::from(cache.world_positions[BoneId::RightShin.index()]), // right_foot
        ];

        let mut positions = Vec::with_capacity(joints.len() * 2);
        for joint in &joints {
            let world_pos = glam::Vec4::new(joint.x, joint.y, joint.z, 1.0);
            let clip_pos = view_proj * world_pos;

            if clip_pos.w > 0.0 {
                let ndc_x = clip_pos.x / clip_pos.w;
                let ndc_y = clip_pos.y / clip_pos.w;
                let screen_x = (ndc_x + 1.0) * 0.5 * width;
                let screen_y = (1.0 - ndc_y) * 0.5 * height;
                positions.push(screen_x);
                positions.push(screen_y);
            } else {
                positions.push(-1000.0);
                positions.push(-1000.0);
            }
        }
        Some(positions)
    })
    .flatten()
    .unwrap_or_default()
}

/// Helper to project a 3D point to screen, offset it, and unproject back
fn project_and_offset(
    pos: glam::Vec3,
    dx: f32,
    dy: f32,
    width: f32,
    height: f32,
    view: glam::Mat4,
    proj: glam::Mat4,
) -> glam::Vec3 {
    let view_proj = proj * view;
    let ndc_pos = view_proj.project_point3(pos);
    let screen_x = (ndc_pos.x + 1.0) * 0.5 * width;
    let screen_y = (1.0 - ndc_pos.y) * 0.5 * height;
    let target_screen_x = screen_x + dx;
    let target_screen_y = screen_y + dy;
    let target_ndc_x = (target_screen_x / (width * 0.5)) - 1.0;
    let target_ndc_y = 1.0 - (target_screen_y / (height * 0.5));
    let target_ndc = glam::Vec3::new(target_ndc_x, target_ndc_y, ndc_pos.z);
    let inverse_vp = view_proj.inverse();
    let clip_pos = glam::Vec4::new(target_ndc.x, target_ndc.y, target_ndc.z, 1.0);
    let mut world_pos = inverse_vp * clip_pos;
    if world_pos.w != 0.0 {
        world_pos /= world_pos.w;
    }
    glam::Vec3::new(world_pos.x, world_pos.y, world_pos.z)
}
