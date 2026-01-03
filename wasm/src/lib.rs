#![cfg_attr(feature = "portable_simd", feature(portable_simd))]

pub mod animation;
#[cfg(target_arch = "wasm32")]
mod bench;
pub mod bone;

/// Backwards compatibility alias for bone_hierarchy -> bone
pub use bone as bone_hierarchy;
pub use bone::AnimationId;
pub mod camera;

#[cfg(target_arch = "wasm32")]
pub mod editor;
#[cfg(test)]
pub mod generator;
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

pub use glam::Vec3;

// Re-exports for WASM API
#[cfg(target_arch = "wasm32")]
pub use gpu::init_gpu;
#[cfg(target_arch = "wasm32")]
pub use state::App;

pub use math::Mat4;
pub use math::Mat4Extended;

use crate::animation::{sample_animation, AnimationLibrary, PlaybackState};
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

// App methods for skeleton updates
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl App {
    /// Update skeleton from the active editor session
    /// Call this every frame before render_frame() when editing
    pub fn update_skeleton_from_session(&self) {
        if let Some(session) = self.state.editor() {
            let matrices = compute_matrices_from_session(session);
            self.update_bone_uniforms(&matrices);
        }
    }

    /// Update skeleton from the current animation playback state
    /// Call this every frame before render_frame() for non-editor mode
    pub fn update_skeleton_from_playback(&self) {
        let matrices =
            compute_matrices_from_playback(&self.state.animation_library, &self.state.playback);
        self.update_bone_uniforms(&matrices);
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
