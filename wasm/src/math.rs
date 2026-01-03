//! Linear algebra primitives using glam with preserved handwritten kernels.

pub use glam::Mat4;
// Extension of glam Mat4, with hand written kernels for multiplication and transpose
pub trait Mat4Extended {
    /// Scalar implementation of matrix multiplication
    fn multiply_scalar(&self, other: &Mat4) -> Mat4;

    /// Portable SIMD implementation (requires nightly + portable_simd feature)
    #[cfg(feature = "portable_simd")]
    fn multiply_std_simd(&self, other: &Mat4) -> Mat4;

    /// WebAssembly Relaxed SIMD implementation (requires relaxed-simd target feature)
    #[cfg(all(target_arch = "wasm32", target_feature = "relaxed-simd"))]
    fn multiply_relaxed_simd(&self, other: &Mat4) -> Mat4;

    /// Dispatcher for the best available handwritten multiplication kernel
    fn multiply_fast(&self, other: &Mat4) -> Mat4;

    /// Scalar implementation of matrix transpose
    fn transpose_scalar(&self) -> Mat4;

    /// Portable SIMD implementation of matrix transpose
    #[cfg(feature = "portable_simd")]
    fn transpose_std_simd(&self) -> Mat4;

    /// WebAssembly Relaxed SIMD implementation of matrix transpose
    #[cfg(all(target_arch = "wasm32", target_feature = "relaxed-simd"))]
    fn transpose_relaxed_simd(&self) -> Mat4;

    /// Dispatcher for the best available handwritten transpose kernel
    fn transpose_fast(&self) -> Mat4;
}

impl Mat4Extended for Mat4 {
    fn multiply_scalar(&self, other: &Mat4) -> Mat4 {
        let mut result_data = [0.0f32; 16];
        let a = self.as_ref();
        let b = other.as_ref();

        for col in 0..4 {
            for row in 0..4 {
                let mut sum = 0.0;
                for k in 0..4 {
                    sum += a[k * 4 + row] * b[col * 4 + k];
                }
                result_data[col * 4 + row] = sum;
            }
        }
        Mat4::from_cols_array(&result_data)
    }

    #[cfg(feature = "portable_simd")]
    fn multiply_std_simd(&self, other: &Mat4) -> Mat4 {
        use std::simd::prelude::*;
        let mut result_data = [0.0f32; 16];
        let a = self.as_ref();
        let b = other.as_ref();

        let a0 = f32x4::from_slice(&a[0..4]);
        let a1 = f32x4::from_slice(&a[4..8]);
        let a2 = f32x4::from_slice(&a[8..12]);
        let a3 = f32x4::from_slice(&a[12..16]);

        for col in 0..4 {
            let b_s0 = f32x4::splat(b[col * 4 + 0]);
            let b_s1 = f32x4::splat(b[col * 4 + 1]);
            let b_s2 = f32x4::splat(b[col * 4 + 2]);
            let b_s3 = f32x4::splat(b[col * 4 + 3]);

            let r = (b_s0 * a0) + (b_s1 * a1) + (b_s2 * a2) + (b_s3 * a3);
            r.copy_to_slice(&mut result_data[col * 4..col * 4 + 4]);
        }
        Mat4::from_cols_array(&result_data)
    }

    #[cfg(all(target_arch = "wasm32", target_feature = "relaxed-simd"))]
    fn multiply_relaxed_simd(&self, other: &Mat4) -> Mat4 {
        use core::arch::wasm32::*;
        let mut result_data = [0.0f32; 16];
        let a = self.as_ref();
        let b = other.as_ref();

        unsafe {
            let a0 = v128_load(a.as_ptr() as *const v128);
            let a1 = v128_load(a.as_ptr().add(4) as *const v128);
            let a2 = v128_load(a.as_ptr().add(8) as *const v128);
            let a3 = v128_load(a.as_ptr().add(12) as *const v128);

            for col in 0..4 {
                let b_ptr = b.as_ptr().add(col * 4);
                let b_s0 = v128_load32_splat(b_ptr as *const u32);
                let b_s1 = v128_load32_splat(b_ptr.add(1) as *const u32);
                let b_s2 = v128_load32_splat(b_ptr.add(2) as *const u32);
                let b_s3 = v128_load32_splat(b_ptr.add(3) as *const u32);

                let mut r = f32x4_mul(b_s0, a0);
                r = f32x4_relaxed_madd(b_s1, a1, r);
                r = f32x4_relaxed_madd(b_s2, a2, r);
                r = f32x4_relaxed_madd(b_s3, a3, r);

                v128_store(result_data.as_mut_ptr().add(col * 4) as *mut v128, r);
            }
        }
        Mat4::from_cols_array(&result_data)
    }

    #[inline(always)]
    fn multiply_fast(&self, other: &Mat4) -> Mat4 {
        cfg_if::cfg_if! {
            if #[cfg(all(target_arch = "wasm32", target_feature = "relaxed-simd"))] {
                self.multiply_relaxed_simd(other)
            } else if #[cfg(feature = "portable_simd")] {
                self.multiply_std_simd(other)
            } else {
                *self * *other
            }
        }
    }

    fn transpose_scalar(&self) -> Mat4 {
        let mut result = [0.0f32; 16];
        let data = self.as_ref();
        for i in 0..4 {
            for j in 0..4 {
                result[i * 4 + j] = data[j * 4 + i];
            }
        }
        Mat4::from_cols_array(&result)
    }

    #[cfg(feature = "portable_simd")]
    fn transpose_std_simd(&self) -> Mat4 {
        use std::simd::{f32x4, simd_swizzle};
        let data = self.as_ref();
        let c0 = f32x4::from_slice(&data[0..4]);
        let c1 = f32x4::from_slice(&data[4..8]);
        let c2 = f32x4::from_slice(&data[8..12]);
        let c3 = f32x4::from_slice(&data[12..16]);

        let t0 = simd_swizzle!(c0, c1, [0, 4, 1, 5]);
        let t1 = simd_swizzle!(c0, c1, [2, 6, 3, 7]);
        let t2 = simd_swizzle!(c2, c3, [0, 4, 1, 5]);
        let t3 = simd_swizzle!(c2, c3, [2, 6, 3, 7]);

        let r0 = simd_swizzle!(t0, t2, [0, 1, 4, 5]);
        let r1 = simd_swizzle!(t0, t2, [2, 3, 6, 7]);
        let r2 = simd_swizzle!(t1, t3, [0, 1, 4, 5]);
        let r3 = simd_swizzle!(t1, t3, [2, 3, 6, 7]);

        let mut result = [0.0f32; 16];
        r0.copy_to_slice(&mut result[0..4]);
        r1.copy_to_slice(&mut result[4..8]);
        r2.copy_to_slice(&mut result[8..12]);
        r3.copy_to_slice(&mut result[12..16]);
        Mat4::from_cols_array(&result)
    }

    #[cfg(all(target_arch = "wasm32", target_feature = "relaxed-simd"))]
    fn transpose_relaxed_simd(&self) -> Mat4 {
        use core::arch::wasm32::*;
        let mut result = [0.0f32; 16];
        let data = self.as_ref();
        unsafe {
            let c0 = v128_load(data.as_ptr() as *const v128);
            let c1 = v128_load(data.as_ptr().add(4) as *const v128);
            let c2 = v128_load(data.as_ptr().add(8) as *const v128);
            let c3 = v128_load(data.as_ptr().add(12) as *const v128);

            let t0 = i32x4_shuffle::<0, 4, 1, 5>(c0, c1);
            let t1 = i32x4_shuffle::<2, 6, 3, 7>(c0, c1);
            let t2 = i32x4_shuffle::<0, 4, 1, 5>(c2, c3);
            let t3 = i32x4_shuffle::<2, 6, 3, 7>(c2, c3);

            let r0 = i32x4_shuffle::<0, 1, 4, 5>(t0, t2);
            let r1 = i32x4_shuffle::<2, 3, 6, 7>(t0, t2);
            let r2 = i32x4_shuffle::<0, 1, 4, 5>(t1, t3);
            let r3 = i32x4_shuffle::<2, 3, 6, 7>(t1, t3);

            v128_store(result.as_mut_ptr() as *mut v128, r0);
            v128_store(result.as_mut_ptr().add(4) as *mut v128, r1);
            v128_store(result.as_mut_ptr().add(8) as *mut v128, r2);
            v128_store(result.as_mut_ptr().add(12) as *mut v128, r3);
        }
        Mat4::from_cols_array(&result)
    }

    #[inline(always)]
    fn transpose_fast(&self) -> Mat4 {
        // All the transposes are just as fast, lets just use the transpose from glam
        self.transpose()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;
    use wasm_bindgen_test::*;

    fn assert_matrix_approx_eq(a: Mat4, b: Mat4) {
        let a_data = a.to_cols_array();
        let b_data = b.to_cols_array();

        let epsilon = if cfg!(all(target_arch = "wasm32", target_feature = "relaxed-simd")) {
            1e-5 // Small tolerance for error if using Relaxed SIMD
        } else {
            0.0
        };

        for i in 0..16 {
            if (a_data[i] - b_data[i]).abs() > epsilon {
                panic!(
                    "Matrix mismatch at index {}: {} vs {} (diff: {})",
                    i,
                    a_data[i],
                    b_data[i],
                    (a_data[i] - b_data[i]).abs()
                );
            }
        }
    }

    #[wasm_bindgen_test]
    fn test_fast_identity_transpose() {
        let mat = Mat4::IDENTITY;
        let transposed = mat.transpose_fast();
        assert_matrix_approx_eq(mat, transposed);
    }

    #[wasm_bindgen_test]
    fn test_fast_transpose() {
        let mut rng = rand::rng();
        for _ in 0..100 {
            // Generate 100 random matrices, transpose them with both glam and fast kernels and compare the results
            let mat = Mat4::from_cols_array(&rng.random::<[f32; 16]>());
            let transposed_fast = mat.transpose_fast();
            let transposed_glam = mat.transpose();
            assert_matrix_approx_eq(transposed_fast, transposed_glam);
        }
    }

    #[wasm_bindgen_test]
    fn test_fast_identity_multiply() {
        let a = Mat4::from_cols_array(&[1.0; 16]);
        let b = Mat4::IDENTITY;
        let multiplied = a.multiply_fast(&b);
        assert_matrix_approx_eq(a, multiplied);
    }

    #[wasm_bindgen_test]
    fn test_fast_multiply() {
        let mut rng = rand::rng();
        for _ in 0..100 {
            // Generate 100 random matrix pairs
            // Multiply them with both glam and
            // fast kernels and compare the results
            let mat = Mat4::from_cols_array(&rng.random::<[f32; 16]>());
            let multiplied_fast = mat.multiply_fast(&mat);
            let multiplied_glam = mat * mat;
            assert_matrix_approx_eq(multiplied_fast, multiplied_glam);
        }
    }
}
