use serde::{Deserialize, Serialize};
use crate::{Result, VectorDbError};

/// Quantization configuration for reducing memory usage
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum QuantizationConfig {
    /// No quantization (original float32)
    None,
    /// Scalar quantization: reduce to int8 (4x memory reduction)
    Scalar(ScalarQuantizationConfig),
    /// Product quantization: compress vectors into codes (8-64x reduction)
    Product(ProductQuantizationConfig),
    /// Binary quantization: 1-bit per dimension (32x reduction + 40x speedup)
    Binary(BinaryQuantizationConfig),
}

/// Scalar quantization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalarQuantizationConfig {
    /// Quantization type
    #[serde(default = "default_scalar_type")]
    pub quantization_type: ScalarType,
    /// Always keep original vectors for rescoring
    #[serde(default = "default_true")]
    pub always_ram: bool,
}

fn default_scalar_type() -> ScalarType {
    ScalarType::Int8
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScalarType {
    Int8,
}

/// Product quantization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductQuantizationConfig {
    /// Number of sub-vectors (must divide vector dimension)
    pub num_segments: usize,
    /// Number of centroids per segment (typically 256 for uint8)
    pub num_centroids: usize,
    /// Use compressed representation in RAM
    #[serde(default = "default_true")]
    pub compression: bool,
    /// Always keep original vectors for rescoring
    #[serde(default)]
    pub always_ram: bool,
}

/// Binary quantization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryQuantizationConfig {
    /// Always keep original vectors for rescoring (recommended)
    #[serde(default = "default_true")]
    pub always_ram: bool,
}

/// Quantized vector representation
#[derive(Debug, Clone)]
pub enum QuantizedVector {
    /// Original float32 vector
    Float32(Vec<f32>),
    /// Scalar quantized (int8)
    ScalarInt8 {
        codes: Vec<i8>,
        min: f32,
        max: f32,
    },
    /// Product quantized
    Product {
        codes: Vec<u8>,
        codebook: Vec<Vec<Vec<f32>>>,
    },
    /// Binary quantized (packed bits)
    Binary {
        bits: Vec<u8>,
        mean: f32,
    },
}

impl QuantizedVector {
    /// Create scalar quantized vector from float32
    pub fn scalar_quantize(vector: &[f32]) -> Self {
        let min = vector.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = vector.iter().cloned().fold(f32::NEG_INFINITY, f32::max);

        let range = max - min;
        let scale = if range > 0.0 { 255.0 / range } else { 1.0 };

        let codes: Vec<i8> = vector
            .iter()
            .map(|&v| {
                let normalized = ((v - min) * scale) as i32;
                (normalized.clamp(0, 255) as i8).wrapping_sub(128)
            })
            .collect();

        QuantizedVector::ScalarInt8 { codes, min, max }
    }

    /// Dequantize scalar int8 back to float32
    pub fn scalar_dequantize(&self) -> Vec<f32> {
        match self {
            QuantizedVector::ScalarInt8 { codes, min, max } => {
                let range = max - min;
                let scale = range / 255.0;

                codes
                    .iter()
                    .map(|&code| {
                        let normalized = (code.wrapping_add(128)) as f32;
                        min + normalized * scale
                    })
                    .collect()
            }
            QuantizedVector::Float32(v) => v.clone(),
            _ => vec![],
        }
    }

    /// Create binary quantized vector from float32
    pub fn binary_quantize(vector: &[f32]) -> Self {
        // Calculate mean
        let mean: f32 = vector.iter().sum::<f32>() / vector.len() as f32;

        // Convert to bits: 1 if above mean, 0 otherwise
        let num_bytes = (vector.len() + 7) / 8;
        let mut bits = vec![0u8; num_bytes];

        for (i, &value) in vector.iter().enumerate() {
            if value > mean {
                let byte_idx = i / 8;
                let bit_idx = i % 8;
                bits[byte_idx] |= 1 << bit_idx;
            }
        }

        QuantizedVector::Binary { bits, mean }
    }

    /// Reconstruct approximate float32 from binary quantization
    pub fn binary_dequantize(&self, dimension: usize) -> Vec<f32> {
        match self {
            QuantizedVector::Binary { bits, mean } => {
                let mut result = Vec::with_capacity(dimension);

                for i in 0..dimension {
                    let byte_idx = i / 8;
                    let bit_idx = i % 8;
                    let bit = (bits[byte_idx] >> bit_idx) & 1;

                    // Use mean + stddev approximation
                    // If bit is 1, value is above mean, otherwise below
                    result.push(if bit == 1 { *mean + 1.0 } else { *mean - 1.0 });
                }

                result
            }
            QuantizedVector::Float32(v) => v.clone(),
            _ => vec![],
        }
    }

    /// Get memory size in bytes
    pub fn memory_size(&self) -> usize {
        match self {
            QuantizedVector::Float32(v) => v.len() * 4,
            QuantizedVector::ScalarInt8 { codes, .. } => codes.len() + 8, // codes + min/max
            QuantizedVector::Product { codes, .. } => codes.len() + 4, // Approximate
            QuantizedVector::Binary { bits, .. } => bits.len() + 4, // bits + mean
        }
    }
}

/// Fast distance computation for quantized vectors
pub fn quantized_distance(
    a: &QuantizedVector,
    b: &QuantizedVector,
    distance_type: QuantizedDistanceType,
) -> f32 {
    match (a, b) {
        (QuantizedVector::ScalarInt8 { codes: a_codes, .. },
         QuantizedVector::ScalarInt8 { codes: b_codes, .. }) => {
            scalar_int8_distance(a_codes, b_codes, distance_type)
        }
        (QuantizedVector::Binary { bits: a_bits, .. },
         QuantizedVector::Binary { bits: b_bits, .. }) => {
            binary_hamming_distance(a_bits, b_bits)
        }
        (QuantizedVector::Float32(a_vec), QuantizedVector::Float32(b_vec)) => {
            // Fall back to regular distance computation
            match distance_type {
                QuantizedDistanceType::Dot => {
                    -crate::distance::dot_product(a_vec, b_vec)
                }
                QuantizedDistanceType::Cosine => {
                    1.0 - crate::distance::cosine_similarity(a_vec, b_vec)
                }
                QuantizedDistanceType::Euclidean => {
                    crate::distance::euclidean_distance(a_vec, b_vec)
                }
            }
        }
        _ => {
            // Mixed types - dequantize and compute
            let a_vec = match a {
                QuantizedVector::ScalarInt8 { .. } => a.scalar_dequantize(),
                QuantizedVector::Binary { .. } => a.binary_dequantize(256), // Default dim
                QuantizedVector::Float32(v) => v.clone(),
                _ => vec![],
            };

            let b_vec = match b {
                QuantizedVector::ScalarInt8 { .. } => b.scalar_dequantize(),
                QuantizedVector::Binary { .. } => b.binary_dequantize(256),
                QuantizedVector::Float32(v) => v.clone(),
                _ => vec![],
            };

            match distance_type {
                QuantizedDistanceType::Dot => {
                    -crate::distance::dot_product(&a_vec, &b_vec)
                }
                QuantizedDistanceType::Cosine => {
                    1.0 - crate::distance::cosine_similarity(&a_vec, &b_vec)
                }
                QuantizedDistanceType::Euclidean => {
                    crate::distance::euclidean_distance(&a_vec, &b_vec)
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum QuantizedDistanceType {
    Dot,
    Cosine,
    Euclidean,
}

/// Fast int8 distance computation
fn scalar_int8_distance(
    a: &[i8],
    b: &[i8],
    distance_type: QuantizedDistanceType,
) -> f32 {
    match distance_type {
        QuantizedDistanceType::Dot => {
            // Negative because we want maximum dot product
            let dot: i32 = a.iter().zip(b.iter())
                .map(|(&ai, &bi)| ai as i32 * bi as i32)
                .sum();
            -(dot as f32)
        }
        QuantizedDistanceType::Euclidean => {
            let sum: i32 = a.iter().zip(b.iter())
                .map(|(&ai, &bi)| {
                    let diff = ai as i32 - bi as i32;
                    diff * diff
                })
                .sum();
            (sum as f32).sqrt()
        }
        QuantizedDistanceType::Cosine => {
            // Approximate cosine using int8
            let dot: i32 = a.iter().zip(b.iter())
                .map(|(&ai, &bi)| ai as i32 * bi as i32)
                .sum();

            let a_norm: i32 = a.iter()
                .map(|&ai| (ai as i32) * (ai as i32))
                .sum();
            let b_norm: i32 = b.iter()
                .map(|&bi| (bi as i32) * (bi as i32))
                .sum();

            let denominator = ((a_norm as f32) * (b_norm as f32)).sqrt();
            if denominator > 0.0 {
                1.0 - (dot as f32 / denominator)
            } else {
                1.0
            }
        }
    }
}

/// Ultra-fast binary hamming distance using bit operations
fn binary_hamming_distance(a: &[u8], b: &[u8]) -> f32 {
    let mut distance = 0u32;

    for (&a_byte, &b_byte) in a.iter().zip(b.iter()) {
        // XOR to find differing bits, then count them
        distance += (a_byte ^ b_byte).count_ones();
    }

    distance as f32
}

/// Product Quantization encoder
pub struct ProductQuantizer {
    num_segments: usize,
    num_centroids: usize,
    dimension: usize,
    segment_size: usize,
    codebooks: Vec<Vec<Vec<f32>>>,
}

impl ProductQuantizer {
    /// Create a new product quantizer
    pub fn new(dimension: usize, num_segments: usize, num_centroids: usize) -> Result<Self> {
        if dimension % num_segments != 0 {
            return Err(VectorDbError::Configuration {
                message: format!(
                    "Dimension {} must be divisible by num_segments {}",
                    dimension, num_segments
                ),
            });
        }

        let segment_size = dimension / num_segments;

        Ok(Self {
            num_segments,
            num_centroids,
            dimension,
            segment_size,
            codebooks: vec![],
        })
    }

    /// Train codebooks on a set of vectors
    pub fn train(&mut self, vectors: &[Vec<f32>]) -> Result<()> {
        if vectors.is_empty() {
            return Err(VectorDbError::Configuration {
                message: "Cannot train on empty vector set".to_string(),
            });
        }

        self.codebooks = vec![vec![]; self.num_segments];

        // For each segment, run k-means to find centroids
        for segment_idx in 0..self.num_segments {
            let start_dim = segment_idx * self.segment_size;
            let end_dim = start_dim + self.segment_size;

            // Extract sub-vectors for this segment
            let sub_vectors: Vec<Vec<f32>> = vectors
                .iter()
                .map(|v| v[start_dim..end_dim].to_vec())
                .collect();

            // Simple k-means (for production, use more sophisticated clustering)
            let centroids = simple_kmeans(&sub_vectors, self.num_centroids, 10)?;
            self.codebooks[segment_idx] = centroids;
        }

        Ok(())
    }

    /// Encode a vector using product quantization
    pub fn encode(&self, vector: &[f32]) -> Result<Vec<u8>> {
        if vector.len() != self.dimension {
            return Err(VectorDbError::InvalidDimension {
                expected: self.dimension,
                actual: vector.len(),
            });
        }

        let mut codes = Vec::with_capacity(self.num_segments);

        for segment_idx in 0..self.num_segments {
            let start_dim = segment_idx * self.segment_size;
            let end_dim = start_dim + self.segment_size;
            let sub_vector = &vector[start_dim..end_dim];

            // Find nearest centroid
            let code = self.find_nearest_centroid(segment_idx, sub_vector);
            codes.push(code as u8);
        }

        Ok(codes)
    }

    fn find_nearest_centroid(&self, segment_idx: usize, sub_vector: &[f32]) -> usize {
        let centroids = &self.codebooks[segment_idx];
        let mut min_distance = f32::INFINITY;
        let mut nearest_idx = 0;

        for (idx, centroid) in centroids.iter().enumerate() {
            let distance = crate::distance::euclidean_distance(sub_vector, centroid);
            if distance < min_distance {
                min_distance = distance;
                nearest_idx = idx;
            }
        }

        nearest_idx
    }

    /// Decode codes back to approximate vector
    pub fn decode(&self, codes: &[u8]) -> Vec<f32> {
        let mut result = Vec::with_capacity(self.dimension);

        for (segment_idx, &code) in codes.iter().enumerate() {
            let centroid = &self.codebooks[segment_idx][code as usize];
            result.extend_from_slice(centroid);
        }

        result
    }
}

/// Simple k-means clustering for PQ
fn simple_kmeans(vectors: &[Vec<f32>], k: usize, max_iterations: usize) -> Result<Vec<Vec<f32>>> {
    if vectors.is_empty() || k == 0 {
        return Err(VectorDbError::Configuration {
            message: "Invalid k-means parameters".to_string(),
        });
    }

    let dim = vectors[0].len();

    // Initialize centroids randomly
    let mut centroids: Vec<Vec<f32>> = vectors
        .iter()
        .step_by(vectors.len() / k.min(vectors.len()))
        .take(k)
        .cloned()
        .collect();

    // Ensure we have k centroids
    while centroids.len() < k {
        centroids.push(vectors[0].clone());
    }

    for _ in 0..max_iterations {
        // Assign points to nearest centroid
        let mut clusters: Vec<Vec<Vec<f32>>> = vec![vec![]; k];

        for vector in vectors {
            let mut min_dist = f32::INFINITY;
            let mut nearest_idx = 0;

            for (idx, centroid) in centroids.iter().enumerate() {
                let dist = crate::distance::euclidean_distance(vector, centroid);
                if dist < min_dist {
                    min_dist = dist;
                    nearest_idx = idx;
                }
            }

            clusters[nearest_idx].push(vector.clone());
        }

        // Update centroids
        let mut changed = false;
        for (idx, cluster) in clusters.iter().enumerate() {
            if !cluster.is_empty() {
                let mut new_centroid = vec![0.0; dim];
                for vector in cluster {
                    for (i, &v) in vector.iter().enumerate() {
                        new_centroid[i] += v;
                    }
                }
                for v in &mut new_centroid {
                    *v /= cluster.len() as f32;
                }

                if !vectors_equal(&centroids[idx], &new_centroid) {
                    centroids[idx] = new_centroid;
                    changed = true;
                }
            }
        }

        if !changed {
            break;
        }
    }

    Ok(centroids)
}

fn vectors_equal(a: &[f32], b: &[f32]) -> bool {
    a.iter().zip(b.iter()).all(|(x, y)| (x - y).abs() < 1e-6)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scalar_quantization() {
        let vector = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let quantized = QuantizedVector::scalar_quantize(&vector);
        let dequantized = quantized.scalar_dequantize();

        // Check approximate reconstruction
        for (original, reconstructed) in vector.iter().zip(dequantized.iter()) {
            assert!((original - reconstructed).abs() < 0.1);
        }
    }

    #[test]
    fn test_binary_quantization() {
        let vector = vec![1.0, -1.0, 2.0, -2.0, 0.5, -0.5];
        let quantized = QuantizedVector::binary_quantize(&vector);

        // Binary quantization should use much less memory
        assert!(quantized.memory_size() < vector.len() * 4);
    }

    #[test]
    fn test_binary_distance() {
        let a = vec![1.0, 1.0, -1.0, -1.0];
        let b = vec![1.0, -1.0, 1.0, -1.0];

        let qa = QuantizedVector::binary_quantize(&a);
        let qb = QuantizedVector::binary_quantize(&b);

        let distance = quantized_distance(&qa, &qb, QuantizedDistanceType::Euclidean);
        assert!(distance >= 0.0);
    }

    #[test]
    fn test_product_quantization() {
        let mut pq = ProductQuantizer::new(128, 4, 256).unwrap();

        // Create some training vectors
        let training_vectors: Vec<Vec<f32>> = (0..100)
            .map(|_| (0..128).map(|_| rand::random::<f32>()).collect())
            .collect();

        pq.train(&training_vectors).unwrap();

        let test_vector: Vec<f32> = (0..128).map(|_| rand::random::<f32>()).collect();
        let codes = pq.encode(&test_vector).unwrap();

        // Codes should be much smaller than original
        assert_eq!(codes.len(), 4); // 4 segments
        assert!(codes.len() < test_vector.len() * 4);
    }
}
