//! Jokkerin Ventti WebGPU Engine - Wasm Core
//!
//! WebGPU rendering from Rust using wgpu and wasm-bindgen.

#![cfg_attr(feature = "portable_simd", feature(portable_simd))]

#[cfg(target_arch = "wasm32")]
mod bench;
#[cfg(target_arch = "wasm32")]
mod gpu;
mod math;
mod skeleton;
mod skeleton_constants;

use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
pub use bench::run_benchmarks;
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
