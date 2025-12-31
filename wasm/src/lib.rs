//! Jokkerin Ventti WebGPU Engine - Wasm Core
//!
//! WebGPU rendering from Rust using wgpu and wasm-bindgen.

#![cfg_attr(feature = "portable_simd", feature(portable_simd))]

pub mod animation;
#[cfg(target_arch = "wasm32")]
mod bench;
mod bone_hierarchy;
pub mod camera;

#[cfg(target_arch = "wasm32")]
pub mod editor;
#[cfg(target_arch = "wasm32")]
pub mod gpu;
pub mod ik;
mod math;
mod skeleton;
mod skeleton_constants;

use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
pub use bench::run_benchmarks;

// Re-exports for WASM API
#[cfg(target_arch = "wasm32")]
pub use camera::{get_camera_right_axis, rotate_camera, update_camera};

#[cfg(target_arch = "wasm32")]
pub use animation::{advance_time, load_animation, set_exercise};

// Handle-based editor functions
#[cfg(target_arch = "wasm32")]
pub use editor::{
    JointInfo, add_keyframe, create_editor_session, delete_keyframe, destroy_editor_session,
    drag_joint, export_clip_json, get_bone_info, get_joint_positions, get_keyframe_count,
    get_keyframe_index, get_keyframe_time, set_bone_position, set_bone_rotation,
    set_keyframe_index,
};
pub use glam::Vec3;
#[cfg(target_arch = "wasm32")]
pub use gpu::{
    get_current_projection_matrix, get_current_view_matrix, init_gpu, render_frame, resize_surface,
    sync_camera,
};

pub use math::Mat4;
pub use math::Mat4Extended;

/// Update skeleton from a handle-based editor session
/// Call this every frame before render_frame() when using the new handle-based API
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn update_skeleton_from_session(handle: u32) {
    use crate::bone_hierarchy::RotationPose;
    use crate::editor::{EditorHandle, with_session_ref};

    let pose = with_session_ref(handle as EditorHandle, |session| {
        session
            .clip
            .keyframes
            .get(session.keyframe_index)
            .map(|kf| kf.pose.clone())
            .unwrap_or_else(RotationPose::bind_pose)
    });

    let mut pose = pose.unwrap_or_else(RotationPose::bind_pose);
    pose = pose.apply_floor_constraint();
    let skeleton = pose.to_skeleton();
    let matrices = skeleton.compute_bone_matrices();

    gpu::update_bone_uniforms(&matrices);
}

/// Update skeleton from the current animation playback state
/// Call this every frame before render_frame() for non-editor mode
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn update_skeleton_from_playback() {
    // Get playback state
    let playback = crate::animation::PLAYBACK_STATE.with(|p| p.borrow().clone());

    // Sample animation from library (pure function)
    let mut pose = crate::animation::ANIMATION_LIBRARY.with(|lib| {
        let library = lib.borrow();
        crate::animation::sample_animation(&library, &playback)
    });

    // Apply floor constraint (keeps hips above floor)
    pose = pose.apply_floor_constraint();

    let skeleton = pose.to_skeleton();
    let matrices = skeleton.compute_bone_matrices();

    gpu::update_bone_uniforms(&matrices);
}

/// Log to browser console
#[wasm_bindgen]
pub fn log(msg: &str) {
    log::info!("{}", msg);
}

/// Simple test function
#[wasm_bindgen]
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
mod tests {
    use wasm_bindgen_test::*;
    wasm_bindgen_test_configure!(run_in_browser);
}
