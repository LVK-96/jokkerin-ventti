//! Jokkerin Ventti WebGPU Engine - Wasm Core
//!
//! WebGPU rendering from Rust using wgpu and wasm-bindgen.

#![cfg_attr(feature = "portable_simd", feature(portable_simd))]

#[cfg(target_arch = "wasm32")]
mod bench;
mod bone_hierarchy;

pub mod editor;
#[cfg(target_arch = "wasm32")]
pub mod gpu;
pub mod ik;
mod math;
mod skeleton;
mod skeleton_constants;

use std::cell::RefCell;
use wasm_bindgen::prelude::*;

// Shared global state
thread_local! {
    pub static GPU_STATE: RefCell<Option<gpu::GpuState>> = const { RefCell::new(None) };
}

#[cfg(target_arch = "wasm32")]
pub use bench::run_benchmarks;
#[cfg(target_arch = "wasm32")]
pub use editor::{
    add_keyframe_copy, apply_joint_drag, enter_editor_mode, exit_editor_mode,
    export_animation_json, get_animation_keyframe_count, get_current_keyframe_time, get_joint_info,
    get_joint_screen_positions, remove_keyframe, set_editor_keyframe, set_joint_position_editor,
    set_joint_rotation,
};
pub use glam::Vec3;
#[cfg(target_arch = "wasm32")]
pub use gpu::{
    init_gpu, load_animation, render_frame, resize_gpu, set_exercise, update_skeleton,
    update_time_uniform,
};
pub use math::Mat4;
pub use math::Mat4Extended;

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
