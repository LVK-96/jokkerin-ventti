use crate::math::{Mat4, Mat4Handwritten};
use wasm_bindgen::prelude::*;

#[derive(serde::Serialize)]
pub struct BenchmarkResults {
    pub iterations: i32,
    pub scalar_ms: f64,
    pub simd_std_ms: Option<f64>,
    pub fma_relaxed_ms: Option<f64>,
    pub fma_supported: bool,
    pub glam_ms: f64,
    // Transpose
    pub transpose_scalar_ms: f64,
    pub transpose_std_ms: Option<f64>,
    pub transpose_relaxed_ms: Option<f64>,
    pub transpose_glam_ms: f64,
}

/// Run performance comparison between different Matrix Multiply implementations
#[wasm_bindgen]
pub fn run_benchmarks(iterations: i32) -> JsValue {
    use std::hint::black_box;

    let window = web_sys::window().expect("should have a window");
    let perf = window.performance().expect("should have performance");
    let m1 = black_box(Mat4::IDENTITY);
    let m2 = black_box(Mat4::IDENTITY);

    // Warm-up to trigger JIT
    for _ in 0..10_000 {
        black_box(m1.multiply_scalar(&m2));
        black_box(m1 * m2);
        black_box(m1.transpose_scalar());
        black_box(m1.transpose());
        #[cfg(feature = "portable_simd")]
        {
            black_box(m1.multiply_std_simd(&m2));
            black_box(m1.transpose_std_simd());
        }
        #[cfg(all(target_arch = "wasm32", target_feature = "relaxed-simd"))]
        {
            black_box(m1.multiply_relaxed_simd(&m2));
            black_box(m1.transpose_relaxed_simd());
        }
    }

    // 1. Scalar (Handwritten)
    let start = perf.now();
    for _ in 0..iterations {
        black_box(m1.multiply_scalar(&m2));
    }
    let scalar_time = perf.now() - start;

    // 2. Portable SIMD (Handwritten)
    #[cfg(feature = "portable_simd")]
    let simd_time = {
        let start = perf.now();
        for _ in 0..iterations {
            black_box(m1.multiply_std_simd(&m2));
        }
        Some(perf.now() - start)
    };
    #[cfg(not(feature = "portable_simd"))]
    let simd_time: Option<f64> = None;

    // 3. Relaxed SIMD (Handwritten)
    #[cfg(all(target_arch = "wasm32", target_feature = "relaxed-simd"))]
    let fma_result = {
        let start = perf.now();
        for _ in 0..iterations {
            black_box(m1.multiply_relaxed_simd(&m2));
        }
        Some(perf.now() - start)
    };
    #[cfg(not(all(target_arch = "wasm32", target_feature = "relaxed-simd")))]
    let fma_result: Option<f64> = None;

    // 4. Glam (Library implementation)
    let start = perf.now();
    for _ in 0..iterations {
        black_box(m1 * m2);
    }
    let glam_time = perf.now() - start;

    // --- Transpose Benchmarks ---

    // 1. Transpose Scalar
    let start = perf.now();
    for _ in 0..iterations {
        black_box(m1.transpose_scalar());
    }
    let t_scalar_time = perf.now() - start;

    // 2. Transpose std_simd
    #[cfg(feature = "portable_simd")]
    let t_simd_time = {
        let start = perf.now();
        for _ in 0..iterations {
            black_box(m1.transpose_std_simd());
        }
        Some(perf.now() - start)
    };
    #[cfg(not(feature = "portable_simd"))]
    let t_simd_time: Option<f64> = None;

    // 3. Transpose Relaxed SIMD
    #[cfg(all(target_arch = "wasm32", target_feature = "relaxed-simd"))]
    let t_relaxed_time = {
        let start = perf.now();
        for _ in 0..iterations {
            black_box(m1.transpose_relaxed_simd());
        }
        Some(perf.now() - start)
    };
    #[cfg(not(all(target_arch = "wasm32", target_feature = "relaxed-simd")))]
    let t_relaxed_time: Option<f64> = None;

    // 4. Transpose Glam
    let start = perf.now();
    for _ in 0..iterations {
        black_box(m1.transpose());
    }
    let t_glam_time = perf.now() - start;

    let result = BenchmarkResults {
        iterations,
        scalar_ms: scalar_time,
        simd_std_ms: simd_time,
        fma_relaxed_ms: fma_result,
        fma_supported: fma_result.is_some(),
        glam_ms: glam_time,
        transpose_scalar_ms: t_scalar_time,
        transpose_std_ms: t_simd_time,
        transpose_relaxed_ms: t_relaxed_time,
        transpose_glam_ms: t_glam_time,
    };

    serde_wasm_bindgen::to_value(&result).unwrap()
}
