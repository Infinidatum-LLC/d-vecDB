/// SIMD-optimized vector operations
/// Provides 2-4x speedup over scalar operations on modern CPUs

#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::*;

/// Calculate dot product between two vectors using SIMD
pub fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "Vector dimensions must match");

    // Check CPU features and use appropriate SIMD level
    #[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
    {
        unsafe { dot_product_avx2(a, b) }
    }

    #[cfg(all(target_arch = "x86_64", target_feature = "sse2", not(target_feature = "avx2")))]
    {
        unsafe { dot_product_sse2(a, b) }
    }

    #[cfg(not(any(target_feature = "avx2", target_feature = "sse2")))]
    {
        dot_product_scalar(a, b)
    }
}

/// AVX2 SIMD implementation (8 floats at once)
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
#[inline]
unsafe fn dot_product_avx2(a: &[f32], b: &[f32]) -> f32 {
    const LANES: usize = 8;
    let len = a.len();
    let simd_len = len - (len % LANES);

    let mut sum = _mm256_setzero_ps();

    // Process 8 floats at a time
    for i in (0..simd_len).step_by(LANES) {
        let va = _mm256_loadu_ps(a.as_ptr().add(i));
        let vb = _mm256_loadu_ps(b.as_ptr().add(i));
        let prod = _mm256_mul_ps(va, vb);
        sum = _mm256_add_ps(sum, prod);
    }

    // Horizontal sum
    let mut result = {
        let temp = _mm256_hadd_ps(sum, sum);
        let temp = _mm256_hadd_ps(temp, temp);
        let lo = _mm256_castps256_ps128(temp);
        let hi = _mm256_extractf128_ps(temp, 1);
        let sum_vec = _mm_add_ps(lo, hi);
        _mm_cvtss_f32(sum_vec)
    };

    // Handle remaining elements
    for i in simd_len..len {
        result += a[i] * b[i];
    }

    result
}

/// SSE2 SIMD implementation (4 floats at once)
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "sse2")]
#[inline]
unsafe fn dot_product_sse2(a: &[f32], b: &[f32]) -> f32 {
    const LANES: usize = 4;
    let len = a.len();
    let simd_len = len - (len % LANES);

    let mut sum = _mm_setzero_ps();

    // Process 4 floats at a time
    for i in (0..simd_len).step_by(LANES) {
        let va = _mm_loadu_ps(a.as_ptr().add(i));
        let vb = _mm_loadu_ps(b.as_ptr().add(i));
        let prod = _mm_mul_ps(va, vb);
        sum = _mm_add_ps(sum, prod);
    }

    // Horizontal sum
    let mut result = {
        let temp = _mm_hadd_ps(sum, sum);
        let temp = _mm_hadd_ps(temp, temp);
        _mm_cvtss_f32(temp)
    };

    // Handle remaining elements
    for i in simd_len..len {
        result += a[i] * b[i];
    }

    result
}

/// Scalar fallback implementation
#[inline]
fn dot_product_scalar(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| x * y)
        .sum()
}

/// Calculate magnitude of a vector using SIMD
pub fn magnitude(vector: &[f32]) -> f32 {
    dot_product(vector, vector).sqrt()
}

/// Calculate Euclidean distance between two vectors using SIMD
pub fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "Vector dimensions must match");

    #[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
    {
        unsafe { euclidean_distance_avx2(a, b) }
    }

    #[cfg(all(target_arch = "x86_64", target_feature = "sse2", not(target_feature = "avx2")))]
    {
        unsafe { euclidean_distance_sse2(a, b) }
    }

    #[cfg(not(any(target_feature = "avx2", target_feature = "sse2")))]
    {
        euclidean_distance_scalar(a, b)
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
#[inline]
unsafe fn euclidean_distance_avx2(a: &[f32], b: &[f32]) -> f32 {
    const LANES: usize = 8;
    let len = a.len();
    let simd_len = len - (len % LANES);

    let mut sum = _mm256_setzero_ps();

    for i in (0..simd_len).step_by(LANES) {
        let va = _mm256_loadu_ps(a.as_ptr().add(i));
        let vb = _mm256_loadu_ps(b.as_ptr().add(i));
        let diff = _mm256_sub_ps(va, vb);
        let prod = _mm256_mul_ps(diff, diff);
        sum = _mm256_add_ps(sum, prod);
    }

    let mut result = {
        let temp = _mm256_hadd_ps(sum, sum);
        let temp = _mm256_hadd_ps(temp, temp);
        let lo = _mm256_castps256_ps128(temp);
        let hi = _mm256_extractf128_ps(temp, 1);
        let sum_vec = _mm_add_ps(lo, hi);
        _mm_cvtss_f32(sum_vec)
    };

    for i in simd_len..len {
        let diff = a[i] - b[i];
        result += diff * diff;
    }

    result.sqrt()
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "sse2")]
#[inline]
unsafe fn euclidean_distance_sse2(a: &[f32], b: &[f32]) -> f32 {
    const LANES: usize = 4;
    let len = a.len();
    let simd_len = len - (len % LANES);

    let mut sum = _mm_setzero_ps();

    for i in (0..simd_len).step_by(LANES) {
        let va = _mm_loadu_ps(a.as_ptr().add(i));
        let vb = _mm_loadu_ps(b.as_ptr().add(i));
        let diff = _mm_sub_ps(va, vb);
        let prod = _mm_mul_ps(diff, diff);
        sum = _mm_add_ps(sum, prod);
    }

    let mut result = {
        let temp = _mm_hadd_ps(sum, sum);
        let temp = _mm_hadd_ps(temp, temp);
        _mm_cvtss_f32(temp)
    };

    for i in simd_len..len {
        let diff = a[i] - b[i];
        result += diff * diff;
    }

    result.sqrt()
}

#[inline]
fn euclidean_distance_scalar(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| {
            let diff = x - y;
            diff * diff
        })
        .sum::<f32>()
        .sqrt()
}

/// Calculate Manhattan distance between two vectors
pub fn manhattan_distance(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "Vector dimensions must match");

    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).abs())
        .sum()
}

/// Batch distance calculations using rayon for parallelism
/// Calculate distances from one query to many vectors (optimized for cache)
pub fn batch_distances(
    query: &[f32],
    vectors: &[&[f32]],
    metric: crate::types::DistanceMetric,
) -> Vec<f32> {
    use rayon::prelude::*;

    // Process in parallel chunks for better CPU utilization
    vectors.par_iter()
        .map(|vec| crate::distance::distance(query, vec, metric))
        .collect()
}

/// Batch distance matrix calculation
/// Calculate all pairwise distances between two sets of vectors
pub fn distance_matrix(
    queries: &[&[f32]],
    corpus: &[&[f32]],
    metric: crate::types::DistanceMetric,
) -> Vec<Vec<f32>> {
    use rayon::prelude::*;

    queries.par_iter()
        .map(|query| {
            batch_distances(query, corpus, metric)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dot_product() {
        let a = vec![1.0, 2.0, 3.0, 4.0];
        let b = vec![2.0, 3.0, 4.0, 5.0];
        let result = dot_product(&a, &b);
        assert!((result - 40.0).abs() < 1e-5);
    }

    #[test]
    fn test_magnitude() {
        let v = vec![3.0, 4.0];
        let result = magnitude(&v);
        assert!((result - 5.0).abs() < 1e-5);
    }

    #[test]
    fn test_euclidean_distance() {
        let a = vec![0.0, 0.0];
        let b = vec![3.0, 4.0];
        let result = euclidean_distance(&a, &b);
        assert!((result - 5.0).abs() < 1e-5);
    }

    #[test]
    fn test_manhattan_distance() {
        let a = vec![1.0, 2.0];
        let b = vec![4.0, 6.0];
        let result = manhattan_distance(&a, &b);
        assert!((result - 7.0).abs() < 1e-5);
    }

    #[test]
    fn test_batch_distances() {
        let query = vec![1.0, 0.0, 0.0];
        let vectors = vec![
            vec![1.0, 0.0, 0.0],
            vec![0.0, 1.0, 0.0],
            vec![0.0, 0.0, 1.0],
        ];
        let vec_refs: Vec<&[f32]> = vectors.iter().map(|v| v.as_slice()).collect();

        let distances = batch_distances(&query, &vec_refs, crate::types::DistanceMetric::Euclidean);

        assert_eq!(distances.len(), 3);
        assert!(distances[0] < 0.1);  // Close to query
        assert!(distances[1] > 1.0);  // Far from query
    }
}
