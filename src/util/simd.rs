//! SIMD-optimized operations

/// Apply window function to samples using SIMD when available
///
/// Falls back to scalar implementation when SIMD is not available.
#[inline]
pub fn apply_window(samples: &mut [f32], window: &[f32]) {
    #[cfg(all(target_arch = "aarch64", target_feature = "neon"))]
    unsafe {
        apply_window_neon(samples, window);
    }

    #[cfg(all(target_arch = "x86_64", target_feature = "avx"))]
    unsafe {
        apply_window_avx(samples, window);
    }

    #[cfg(not(any(
        all(target_arch = "aarch64", target_feature = "neon"),
        all(target_arch = "x86_64", target_feature = "avx")
    )))]
    {
        apply_window_scalar(samples, window);
    }
}

/// Scalar fallback for window application
#[inline]
fn apply_window_scalar(samples: &mut [f32], window: &[f32]) {
    for (s, w) in samples.iter_mut().zip(window.iter()) {
        *s *= w;
    }
}

/// NEON-optimized window application (ARM64)
#[cfg(all(target_arch = "aarch64", target_feature = "neon"))]
#[target_feature(enable = "neon")]
unsafe fn apply_window_neon(samples: &mut [f32], window: &[f32]) {
    use std::arch::aarch64::*;

    let len = samples.len().min(window.len());
    let chunks = len / 4;

    for i in 0..chunks {
        let offset = i * 4;
        let s = vld1q_f32(samples.as_ptr().add(offset));
        let w = vld1q_f32(window.as_ptr().add(offset));
        let result = vmulq_f32(s, w);
        vst1q_f32(samples.as_mut_ptr().add(offset), result);
    }

    // Handle remaining elements
    for i in (chunks * 4)..len {
        samples[i] *= window[i];
    }
}

/// AVX-optimized window application (x86_64)
#[cfg(all(target_arch = "x86_64", target_feature = "avx"))]
#[target_feature(enable = "avx")]
unsafe fn apply_window_avx(samples: &mut [f32], window: &[f32]) {
    use std::arch::x86_64::*;

    let len = samples.len().min(window.len());
    let chunks = len / 8;

    for i in 0..chunks {
        let offset = i * 8;
        let s = _mm256_loadu_ps(samples.as_ptr().add(offset));
        let w = _mm256_loadu_ps(window.as_ptr().add(offset));
        let result = _mm256_mul_ps(s, w);
        _mm256_storeu_ps(samples.as_mut_ptr().add(offset), result);
    }

    // Handle remaining elements
    for i in (chunks * 8)..len {
        samples[i] *= window[i];
    }
}

/// Calculate magnitude of complex numbers using SIMD
#[inline]
pub fn magnitude_squared(real: &[f32], imag: &[f32], output: &mut [f32]) {
    let len = real.len().min(imag.len()).min(output.len());

    for i in 0..len {
        output[i] = real[i] * real[i] + imag[i] * imag[i];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_window() {
        let mut samples = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let window = vec![0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5];

        apply_window(&mut samples, &window);

        assert!((samples[0] - 0.5).abs() < 0.001);
        assert!((samples[7] - 4.0).abs() < 0.001);
    }

    #[test]
    fn test_magnitude_squared() {
        let real = vec![3.0, 0.0];
        let imag = vec![4.0, 1.0];
        let mut output = vec![0.0, 0.0];

        magnitude_squared(&real, &imag, &mut output);

        assert!((output[0] - 25.0).abs() < 0.001); // 3^2 + 4^2 = 25
        assert!((output[1] - 1.0).abs() < 0.001);  // 0^2 + 1^2 = 1
    }
}
