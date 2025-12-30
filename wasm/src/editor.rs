use crate::bone_hierarchy::{BoneId, RotationAnimationClip};
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};
use wasm_bindgen::prelude::*;

// =============================================================================
// HANDLE PATTERN INFRASTRUCTURE
// =============================================================================

/// Opaque handle to an editor session (passed to/from JavaScript)
pub type EditorHandle = u32;

/// An active editing session with a clip
pub struct EditorSession {
    /// The animation clip being edited (cloned from library)
    pub clip: RotationAnimationClip,
    /// Current keyframe index being edited
    pub keyframe_index: usize,
}

/// Global store for active editor sessions
/// Using Mutex because static requires Sync; safe in single-threaded WASM
static SESSIONS: Mutex<Option<HashMap<EditorHandle, EditorSession>>> = Mutex::new(None);

/// Counter for generating unique handles
static NEXT_HANDLE: AtomicU32 = AtomicU32::new(1);

/// Initialize the sessions HashMap if needed
fn ensure_sessions_init() {
    let mut guard = SESSIONS.lock().unwrap();
    if guard.is_none() {
        *guard = Some(HashMap::new());
    }
}

/// Helper to access a session by handle, executing a closure with mutable access
fn with_session<F, R>(handle: EditorHandle, f: F) -> Option<R>
where
    F: FnOnce(&mut EditorSession) -> R,
{
    let mut guard = SESSIONS.lock().ok()?;
    guard.as_mut()?.get_mut(&handle).map(f)
}

/// Helper to access a session by handle with read-only access
pub fn with_session_ref<F, R>(handle: EditorHandle, f: F) -> Option<R>
where
    F: FnOnce(&EditorSession) -> R,
{
    let guard = SESSIONS.lock().ok()?;
    guard.as_ref()?.get(&handle).map(f)
}

// =============================================================================
// LIFECYCLE FUNCTIONS
// =============================================================================

/// Create a new editor session for the given exercise
/// Returns an opaque handle to use with other editor functions
#[wasm_bindgen]
pub fn create_editor_session(exercise_name: &str) -> EditorHandle {
    ensure_sessions_init();

    let clip = crate::animation::ANIMATION_LIBRARY
        .with(|lib| lib.borrow().get_clip(exercise_name).cloned());

    if let Some(clip) = clip {
        let handle = NEXT_HANDLE.fetch_add(1, Ordering::SeqCst);
        let session = EditorSession {
            clip,
            keyframe_index: 0,
        };

        if let Ok(mut guard) = SESSIONS.lock() {
            if let Some(sessions) = guard.as_mut() {
                sessions.insert(handle, session);
                log::info!("Created editor session {} for: {}", handle, exercise_name);
                return handle;
            }
        }
    } else {
        log::warn!("No animation loaded for exercise: {}", exercise_name);
    }

    0 // Return 0 as invalid handle
}

/// Destroy an editor session and free its resources
#[wasm_bindgen]
pub fn destroy_editor_session(handle: EditorHandle) {
    if let Ok(mut guard) = SESSIONS.lock() {
        if let Some(sessions) = guard.as_mut() {
            if sessions.remove(&handle).is_some() {
                log::info!("Destroyed editor session {}", handle);
            }
        }
    }
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

// =============================================================================
// HANDLE-BASED EDITOR FUNCTIONS
// =============================================================================

/// Get the number of keyframes in a session's clip
#[wasm_bindgen]
pub fn get_keyframe_count(handle: EditorHandle) -> usize {
    with_session_ref(handle, |session| session.clip.keyframes.len()).unwrap_or(0)
}

/// Get the time of the current keyframe
#[wasm_bindgen]
pub fn get_keyframe_time(handle: EditorHandle) -> f32 {
    with_session_ref(handle, |session| {
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
pub fn get_keyframe_index(handle: EditorHandle) -> usize {
    with_session_ref(handle, |session| session.keyframe_index).unwrap_or(0)
}

/// Set the current keyframe index for editing
#[wasm_bindgen]
pub fn set_keyframe_index(handle: EditorHandle, index: usize) {
    with_session(handle, |session| {
        if index < session.clip.keyframes.len() {
            session.keyframe_index = index;
        }
    });
}

/// Add a new keyframe as a copy of the one at after_index
#[wasm_bindgen]
pub fn add_keyframe(handle: EditorHandle, after_index: usize) {
    with_session(handle, |session| {
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
pub fn delete_keyframe(handle: EditorHandle, index: usize) {
    with_session(handle, |session| {
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

/// Export the session's clip as JSON
#[wasm_bindgen]
pub fn export_clip_json(handle: EditorHandle) -> String {
    with_session_ref(handle, |session| {
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
pub fn get_bone_info(handle: EditorHandle, bone_index: usize) -> Option<JointInfo> {
    if bone_index >= crate::bone_hierarchy::BoneId::COUNT {
        return None;
    }

    with_session_ref(handle, |session| {
        let pose = &session.clip.keyframes.get(session.keyframe_index)?.pose;
        let id =
            unsafe { std::mem::transmute::<u8, crate::bone_hierarchy::BoneId>(bone_index as u8) };
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
pub fn set_bone_rotation(handle: EditorHandle, bone_index: usize, rx: f32, ry: f32, rz: f32) {
    if bone_index >= crate::bone_hierarchy::BoneId::COUNT {
        return;
    }

    with_session(handle, |session| {
        if session.keyframe_index < session.clip.keyframes.len() {
            let pose = &mut session.clip.keyframes[session.keyframe_index].pose;
            let id = unsafe {
                std::mem::transmute::<u8, crate::bone_hierarchy::BoneId>(bone_index as u8)
            };
            let q = glam::Quat::from_euler(
                glam::EulerRot::XYZ,
                rx.to_radians(),
                ry.to_radians(),
                rz.to_radians(),
            );
            // Direct mutation - no std::mem::take needed!
            *pose = std::mem::replace(pose, crate::bone_hierarchy::RotationPose::default())
                .with_rotation(id, q);
        }
    });
}

/// Set joint position for a bone using IK/FK in the current keyframe
#[wasm_bindgen]
pub fn set_bone_position(handle: EditorHandle, bone_index: usize, x: f32, y: f32, z: f32) {
    with_session(handle, |session| {
        if session.keyframe_index >= session.clip.keyframes.len() {
            return;
        }

        let pose = &mut session.clip.keyframes[session.keyframe_index].pose;
        let target_pos = glam::Vec3::new(x, y, z);

        if bone_index == 0 {
            // Root position
            *pose = std::mem::replace(pose, crate::bone_hierarchy::RotationPose::default())
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
            *pose = std::mem::replace(pose, crate::bone_hierarchy::RotationPose::default())
                .apply_ik(&chain, target_pos)
                .apply_floor_constraint();
        } else {
            // FK Logic
            let pivot_pos = if let Some(parent) =
                crate::bone_hierarchy::BONE_HIERARCHY[bone_id.index()].parent
            {
                pose.get_position(parent)
            } else {
                pose.root_position
            };

            let target_dir = (target_pos - pivot_pos).normalize_or_zero();
            if target_dir.length_squared() > 1e-6 {
                let parent_rot = compute_world_rotation(pose, bone_id);
                let default_dir = crate::bone_hierarchy::BONE_HIERARCHY[bone_id.index()]
                    .direction
                    .normalize();
                let target_dir_local = parent_rot.inverse() * target_dir;
                let delta_rot = glam::Quat::from_rotation_arc(default_dir, target_dir_local);

                *pose = std::mem::replace(pose, crate::bone_hierarchy::RotationPose::default())
                    .with_rotation(bone_id, delta_rot.normalize());
            }

            // Apply floor constraint
            *pose = std::mem::replace(pose, crate::bone_hierarchy::RotationPose::default())
                .apply_floor_constraint();
        }
    });
}

/// Compute world rotation for a bone's parent (helper for FK)
fn compute_world_rotation(
    pose: &crate::bone_hierarchy::RotationPose,
    bone_id: BoneId,
) -> glam::Quat {
    if let Some(parent) = crate::bone_hierarchy::BONE_HIERARCHY[bone_id.index()].parent {
        let mut rot = pose.local_rotations[parent.index()];
        let mut p = parent;
        while let Some(grandparent) = crate::bone_hierarchy::BONE_HIERARCHY[p.index()].parent {
            rot = pose.local_rotations[grandparent.index()] * rot;
            p = grandparent;
        }
        rot
    } else {
        glam::Quat::IDENTITY
    }
}

/// Apply a screen-space drag to a joint (handle-based version)
#[wasm_bindgen]
pub fn drag_joint(
    handle: EditorHandle,
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

    with_session(handle, |session| {
        if session.keyframe_index >= session.clip.keyframes.len() {
            return;
        }

        let pose = &mut session.clip.keyframes[session.keyframe_index].pose;

        // Root drag
        if joint_index == 0 {
            let current_pos = pose.root_position;
            let target_pos =
                project_and_offset(current_pos, dx, dy, width, height, view_mat, proj_mat);
            *pose = std::mem::replace(pose, crate::bone_hierarchy::RotationPose::default())
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
            *pose = std::mem::replace(pose, crate::bone_hierarchy::RotationPose::default())
                .apply_ik(&chain, target_pos)
                .apply_floor_constraint();
        } else {
            // FK
            let pivot_pos = if let Some(parent) =
                crate::bone_hierarchy::BONE_HIERARCHY[bone_id.index()].parent
            {
                pose.get_position(parent)
            } else {
                pose.root_position
            };

            let target_dir = (target_pos - pivot_pos).normalize_or_zero();
            if target_dir.length_squared() > 1e-6 {
                let parent_rot = compute_world_rotation(pose, bone_id);
                let default_dir = crate::bone_hierarchy::BONE_HIERARCHY[bone_id.index()]
                    .direction
                    .normalize();
                let target_dir_local = parent_rot.inverse() * target_dir;
                let delta_rot = glam::Quat::from_rotation_arc(default_dir, target_dir_local);

                *pose = std::mem::replace(pose, crate::bone_hierarchy::RotationPose::default())
                    .with_rotation(bone_id, delta_rot.normalize());
            }
        }
    });
}

/// Get screen positions of all joints for a session (for picking)
#[wasm_bindgen]
pub fn get_joint_positions(
    handle: EditorHandle,
    view: &[f32],
    proj: &[f32],
    width: f32,
    height: f32,
) -> Vec<f32> {
    if view.len() < 16 || proj.len() < 16 {
        return Vec::new();
    }

    let view_mat = glam::Mat4::from_cols_array(view.try_into().unwrap());
    let proj_mat = glam::Mat4::from_cols_array(proj.try_into().unwrap());
    let view_proj = proj_mat * view_mat;

    with_session_ref(handle, |session| {
        let keyframe = session.clip.keyframes.get(session.keyframe_index)?;
        let skeleton = keyframe.pose.to_skeleton();

        let joints = [
            skeleton.hips,
            skeleton.neck,
            skeleton.neck,
            skeleton.head,
            skeleton.left_shoulder,
            skeleton.left_elbow,
            skeleton.left_hand,
            skeleton.right_shoulder,
            skeleton.right_elbow,
            skeleton.right_hand,
            skeleton.left_hip,
            skeleton.left_knee,
            skeleton.left_foot,
            skeleton.right_hip,
            skeleton.right_knee,
            skeleton.right_foot,
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
