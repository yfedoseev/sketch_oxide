//! Optional SIMD kernels (fable5 doc 02 #2), behind the `simd` feature.
//!
//! Each kernel dispatches at runtime via `is_x86_feature_detected!` and always
//! has a portable scalar fallback, so correctness never depends on the feature
//! or the host CPU — only speed does. The scalar path is what runs by default
//! (the `simd` feature is off) and on non-x86 / pre-AVX2 hardware.

/// Counts the bytes equal to zero in `bytes`.
///
/// Used by HyperLogLog to count empty registers for the small-cardinality
/// linear-counting correction. With the `simd` feature on an AVX2-capable x86-64
/// CPU this compares 32 bytes per instruction; otherwise it falls back to a
/// scalar loop that produces the identical result.
#[must_use]
pub fn count_zero_bytes(bytes: &[u8]) -> usize {
    #[cfg(all(feature = "simd", target_arch = "x86_64"))]
    {
        if std::is_x86_feature_detected!("avx2") {
            // SAFETY: only called after confirming AVX2 is available at runtime.
            #[allow(unsafe_code)]
            return unsafe { count_zero_bytes_avx2(bytes) };
        }
    }
    count_zero_bytes_scalar(bytes)
}

#[inline]
fn count_zero_bytes_scalar(bytes: &[u8]) -> usize {
    bytes.iter().filter(|&&b| b == 0).count()
}

#[cfg(all(feature = "simd", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
#[allow(unsafe_code)]
unsafe fn count_zero_bytes_avx2(bytes: &[u8]) -> usize {
    use std::arch::x86_64::{
        __m256i, _mm256_cmpeq_epi8, _mm256_loadu_si256, _mm256_movemask_epi8, _mm256_setzero_si256,
    };

    let mut count = 0usize;
    let chunks = bytes.chunks_exact(32);
    let remainder = chunks.remainder();
    // SAFETY: this fn is only reached after runtime AVX2 detection; each `chunk`
    // is exactly 32 bytes and `loadu` is unaligned-safe. (Edition 2024 requires
    // the intrinsic calls in an explicit `unsafe` block even inside an unsafe fn.)
    unsafe {
        let zero = _mm256_setzero_si256();
        for chunk in chunks {
            let v = _mm256_loadu_si256(chunk.as_ptr().cast::<__m256i>());
            let eq = _mm256_cmpeq_epi8(v, zero); // 0xFF lane where byte == 0
            let mask = _mm256_movemask_epi8(eq) as u32; // one bit per byte
            count += mask.count_ones() as usize;
        }
    }

    count + count_zero_bytes_scalar(remainder)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_zero_bytes_matches_scalar_reference() {
        // Exercise sizes around the 32-byte SIMD stride boundary and mixed data.
        for len in [0usize, 1, 5, 31, 32, 33, 63, 64, 65, 1000, 4096] {
            let mut v = vec![0u8; len];
            // Set roughly every third byte non-zero, deterministically.
            for (i, b) in v.iter_mut().enumerate() {
                if i % 3 == 0 {
                    *b = (i % 255 + 1) as u8; // 1..=255, never 0
                }
            }
            let expected = v.iter().filter(|&&b| b == 0).count();
            assert_eq!(count_zero_bytes(&v), expected, "len={len}");
        }
        // All-zero and all-nonzero extremes.
        assert_eq!(count_zero_bytes(&[0u8; 100]), 100);
        assert_eq!(count_zero_bytes(&[7u8; 100]), 0);
    }
}
