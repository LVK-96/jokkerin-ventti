#![cfg_attr(feature = "portable_simd", feature(portable_simd))]

pub mod animation;
#[cfg(target_arch = "wasm32")]
mod bench;
pub mod bone;
/// Backwards compatibility alias for bone_hierarchy -> bone
pub use bone as bone_hierarchy;
pub mod camera;

#[cfg(target_arch = "wasm32")]
pub mod editor;
#[cfg(target_arch = "wasm32")]
pub mod gpu;
pub mod ik;
mod math;
pub mod skeleton;
mod skeleton_constants;
#[cfg(target_arch = "wasm32")]
pub mod state;

use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
pub use bench::run_benchmarks;

// Re-exports for WASM API
#[cfg(target_arch = "wasm32")]
pub use camera::{get_camera_right_axis, rotate_camera, update_camera};

#[cfg(target_arch = "wasm32")]
pub use animation::{advance_time, load_animation, set_exercise};

// Editor functions (singleton session)
#[cfg(target_arch = "wasm32")]
pub use editor::{
    JointInfo, add_keyframe, delete_keyframe, drag_joint, export_clip_json, get_bone_info,
    get_joint_positions, get_keyframe_count, get_keyframe_index, get_keyframe_time, is_editing,
    set_bone_position, set_bone_rotation, set_keyframe_index, start_editing, stop_editing,
};
pub use glam::Vec3;
#[cfg(target_arch = "wasm32")]
pub use gpu::{
    get_current_projection_matrix, get_current_view_matrix, init_gpu, render_frame, resize_surface,
    sync_camera,
};

pub use math::Mat4;
pub use math::Mat4Extended;

use crate::animation::{AnimationLibrary, PlaybackState, sample_animation};
#[cfg(target_arch = "wasm32")]
use crate::bone::RotationPose;
use crate::skeleton::RENDER_BONE_COUNT;

/// Compute bone matrices from an editor session's current keyframe
#[cfg(target_arch = "wasm32")]
pub fn compute_matrices_from_session(
    session: &state::EditorSession,
) -> [glam::Mat4; RENDER_BONE_COUNT] {
    let pose = session
        .clip
        .keyframes
        .get(session.keyframe_index)
        .map(|kf| kf.pose.clone())
        .unwrap_or_else(RotationPose::bind_pose);

    let pose = pose.apply_floor_constraint();
    pose.compute_bone_matrices()
}

/// Compute bone matrices from animation playback state
pub fn compute_matrices_from_playback(
    library: &AnimationLibrary,
    playback: &PlaybackState,
) -> [glam::Mat4; RENDER_BONE_COUNT] {
    let pose = sample_animation(library, playback);
    let pose = pose.apply_floor_constraint();
    pose.compute_bone_matrices()
}

/// Update skeleton from the active editor session
/// Call this every frame before render_frame() when editing
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn update_skeleton_from_session() {
    use crate::state::with_app_state;

    let matrices = with_app_state(|app| {
        app.editor()
            .map(|session| compute_matrices_from_session(session))
    })
    .flatten();

    if let Some(matrices) = matrices {
        gpu::update_bone_uniforms(&matrices);
    }
}

/// Update skeleton from the current animation playback state
/// Call this every frame before render_frame() for non-editor mode
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn update_skeleton_from_playback() {
    use crate::state::with_app_state;

    let matrices =
        with_app_state(|app| compute_matrices_from_playback(&app.animation_library, &app.playback));

    if let Some(matrices) = matrices {
        gpu::update_bone_uniforms(&matrices);
    }
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
