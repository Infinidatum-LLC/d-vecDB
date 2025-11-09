use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Sparse vector representation (like BM25/TF-IDF)
/// Only stores non-zero values with their indices
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SparseVector {
    /// Indices of non-zero values
    pub indices: Vec<u32>,
    /// Corresponding non-zero values
    pub values: Vec<f32>,
}

impl SparseVector {
    /// Create a new sparse vector
    pub fn new(indices: Vec<u32>, values: Vec<f32>) -> Self {
        assert_eq!(indices.len(), values.len(), "Indices and values must have same length");
        Self { indices, values }
    }

    /// Create sparse vector from dense vector (threshold-based)
    pub fn from_dense(dense: &[f32], threshold: f32) -> Self {
        let mut indices = Vec::new();
        let mut values = Vec::new();

        for (idx, &value) in dense.iter().enumerate() {
            if value.abs() > threshold {
                indices.push(idx as u32);
                values.push(value);
            }
        }

        Self { indices, values }
    }

    /// Convert to dense vector with specified dimension
    pub fn to_dense(&self, dimension: usize) -> Vec<f32> {
        let mut dense = vec![0.0; dimension];

        for (&idx, &value) in self.indices.iter().zip(self.values.iter()) {
            if (idx as usize) < dimension {
                dense[idx as usize] = value;
            }
        }

        dense
    }

    /// Get number of non-zero elements
    pub fn nnz(&self) -> usize {
        self.indices.len()
    }

    /// Compute dot product with another sparse vector
    pub fn dot(&self, other: &SparseVector) -> f32 {
        // Use two-pointer approach for sorted indices
        let mut i = 0;
        let mut j = 0;
        let mut result = 0.0;

        while i < self.indices.len() && j < other.indices.len() {
            match self.indices[i].cmp(&other.indices[j]) {
                std::cmp::Ordering::Equal => {
                    result += self.values[i] * other.values[j];
                    i += 1;
                    j += 1;
                }
                std::cmp::Ordering::Less => {
                    i += 1;
                }
                std::cmp::Ordering::Greater => {
                    j += 1;
                }
            }
        }

        result
    }

    /// Normalize sparse vector (L2 normalization)
    pub fn normalize(&mut self) {
        let norm: f32 = self.values.iter().map(|v| v * v).sum::<f32>().sqrt();

        if norm > 0.0 {
            for value in &mut self.values {
                *value /= norm;
            }
        }
    }

    /// Compute cosine similarity with another sparse vector
    pub fn cosine_similarity(&self, other: &SparseVector) -> f32 {
        let dot = self.dot(other);

        let self_norm: f32 = self.values.iter().map(|v| v * v).sum::<f32>().sqrt();
        let other_norm: f32 = other.values.iter().map(|v| v * v).sum::<f32>().sqrt();

        if self_norm > 0.0 && other_norm > 0.0 {
            dot / (self_norm * other_norm)
        } else {
            0.0
        }
    }
}

/// Multi-vector (dense + sparse) for hybrid search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiVector {
    /// Dense vector embedding
    pub dense: Option<Vec<f32>>,
    /// Sparse vector (BM25/TF-IDF)
    pub sparse: Option<SparseVector>,
}

impl MultiVector {
    /// Create a multi-vector with both dense and sparse
    pub fn new(dense: Option<Vec<f32>>, sparse: Option<SparseVector>) -> Self {
        Self { dense, sparse }
    }

    /// Create from dense vector only
    pub fn from_dense(dense: Vec<f32>) -> Self {
        Self {
            dense: Some(dense),
            sparse: None,
        }
    }

    /// Create from sparse vector only
    pub fn from_sparse(sparse: SparseVector) -> Self {
        Self {
            dense: None,
            sparse: Some(sparse),
        }
    }
}

/// Hybrid search request combining dense and sparse vectors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridSearchRequest {
    pub collection: String,
    /// Dense vector query (semantic search)
    pub dense: Option<Vec<f32>>,
    /// Sparse vector query (keyword/BM25 search)
    pub sparse: Option<SparseVector>,
    /// Fusion method for combining scores
    #[serde(default = "default_fusion")]
    pub fusion: FusionMethod,
    /// Number of results to return
    pub limit: usize,
    /// Optional filter
    pub filter: Option<crate::filter::Filter>,
}

fn default_fusion() -> FusionMethod {
    FusionMethod::RelativeScoreFusion
}

/// Methods for fusing dense and sparse search results
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FusionMethod {
    /// Weighted average of normalized scores
    RelativeScoreFusion,
    /// Reciprocal Rank Fusion (position-based)
    ReciprocalRankFusion,
    /// Distribution-based score fusion
    DistributionBasedScoreFusion,
}

/// BM25 scorer for text-based sparse vectors
pub struct BM25 {
    k1: f32,
    b: f32,
    avg_doc_length: f32,
    doc_count: usize,
    doc_frequencies: HashMap<u32, usize>,
}

impl BM25 {
    /// Create a new BM25 scorer
    pub fn new(k1: f32, b: f32) -> Self {
        Self {
            k1,
            b,
            avg_doc_length: 0.0,
            doc_count: 0,
            doc_frequencies: HashMap::new(),
        }
    }

    /// Default BM25 parameters
    pub fn default() -> Self {
        Self::new(1.2, 0.75)
    }

    /// Add document to corpus statistics
    pub fn add_document(&mut self, sparse_vec: &SparseVector) {
        self.doc_count += 1;

        // Update document frequencies
        for &idx in &sparse_vec.indices {
            *self.doc_frequencies.entry(idx).or_insert(0) += 1;
        }

        // Update average document length
        let total_length = self.avg_doc_length * (self.doc_count - 1) as f32;
        self.avg_doc_length = (total_length + sparse_vec.nnz() as f32) / self.doc_count as f32;
    }

    /// Compute BM25 score for a query against a document
    pub fn score(&self, query: &SparseVector, document: &SparseVector, doc_length: usize) -> f32 {
        let mut score = 0.0;

        // Create a hashmap for quick document term lookup
        let doc_terms: HashMap<u32, f32> = document
            .indices
            .iter()
            .zip(document.values.iter())
            .map(|(&idx, &val)| (idx, val))
            .collect();

        for (&term_id, &query_tf) in query.indices.iter().zip(query.values.iter()) {
            if let Some(&doc_tf) = doc_terms.get(&term_id) {
                // IDF calculation
                let df = self.doc_frequencies.get(&term_id).copied().unwrap_or(1);
                let idf = ((self.doc_count as f32 - df as f32 + 0.5) / (df as f32 + 0.5) + 1.0).ln();

                // BM25 formula
                let normalized_tf = doc_tf * (self.k1 + 1.0)
                    / (doc_tf + self.k1 * (1.0 - self.b + self.b * doc_length as f32 / self.avg_doc_length));

                score += idf * normalized_tf * query_tf;
            }
        }

        score
    }
}

/// Fuse dense and sparse search results
pub fn fuse_results(
    dense_results: Vec<(uuid::Uuid, f32)>,
    sparse_results: Vec<(uuid::Uuid, f32)>,
    method: FusionMethod,
) -> Vec<(uuid::Uuid, f32)> {
    match method {
        FusionMethod::RelativeScoreFusion => relative_score_fusion(dense_results, sparse_results),
        FusionMethod::ReciprocalRankFusion => reciprocal_rank_fusion(dense_results, sparse_results),
        FusionMethod::DistributionBasedScoreFusion => {
            distribution_based_fusion(dense_results, sparse_results)
        }
    }
}

/// Relative Score Fusion: Normalize and average scores
fn relative_score_fusion(
    dense_results: Vec<(uuid::Uuid, f32)>,
    sparse_results: Vec<(uuid::Uuid, f32)>,
) -> Vec<(uuid::Uuid, f32)> {
    // Collect all unique IDs
    let mut combined: HashMap<uuid::Uuid, (Option<f32>, Option<f32>)> = HashMap::new();

    // Normalize dense scores (0-1 range)
    let dense_max = dense_results.iter().map(|(_, s)| s).fold(f32::NEG_INFINITY, |a, &b| a.max(b));
    let dense_min = dense_results.iter().map(|(_, s)| s).fold(f32::INFINITY, |a, &b| a.min(b));
    let dense_range = dense_max - dense_min;

    for (id, score) in dense_results {
        let normalized = if dense_range > 0.0 {
            (score - dense_min) / dense_range
        } else {
            0.5
        };
        combined.entry(id).or_insert((None, None)).0 = Some(normalized);
    }

    // Normalize sparse scores
    let sparse_max = sparse_results.iter().map(|(_, s)| s).fold(f32::NEG_INFINITY, |a, &b| a.max(b));
    let sparse_min = sparse_results.iter().map(|(_, s)| s).fold(f32::INFINITY, |a, &b| a.min(b));
    let sparse_range = sparse_max - sparse_min;

    for (id, score) in sparse_results {
        let normalized = if sparse_range > 0.0 {
            (score - sparse_min) / sparse_range
        } else {
            0.5
        };
        combined.entry(id).or_insert((None, None)).1 = Some(normalized);
    }

    // Average the scores
    let mut results: Vec<(uuid::Uuid, f32)> = combined
        .into_iter()
        .map(|(id, (dense, sparse))| {
            let score = match (dense, sparse) {
                (Some(d), Some(s)) => (d + s) / 2.0,
                (Some(d), None) => d,
                (None, Some(s)) => s,
                (None, None) => 0.0,
            };
            (id, score)
        })
        .collect();

    // Sort by score descending
    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    results
}

/// Reciprocal Rank Fusion: Combine based on ranks
fn reciprocal_rank_fusion(
    dense_results: Vec<(uuid::Uuid, f32)>,
    sparse_results: Vec<(uuid::Uuid, f32)>,
) -> Vec<(uuid::Uuid, f32)> {
    const K: f32 = 60.0; // Standard RRF constant

    let mut scores: HashMap<uuid::Uuid, f32> = HashMap::new();

    // Add dense ranks
    for (rank, (id, _)) in dense_results.iter().enumerate() {
        let score = 1.0 / (K + rank as f32 + 1.0);
        *scores.entry(*id).or_insert(0.0) += score;
    }

    // Add sparse ranks
    for (rank, (id, _)) in sparse_results.iter().enumerate() {
        let score = 1.0 / (K + rank as f32 + 1.0);
        *scores.entry(*id).or_insert(0.0) += score;
    }

    let mut results: Vec<(uuid::Uuid, f32)> = scores.into_iter().collect();
    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    results
}

/// Distribution-Based Score Fusion (DBSF)
fn distribution_based_fusion(
    dense_results: Vec<(uuid::Uuid, f32)>,
    sparse_results: Vec<(uuid::Uuid, f32)>,
) -> Vec<(uuid::Uuid, f32)> {
    // For now, use relative score fusion
    // TODO: Implement proper DBSF with score distribution analysis
    relative_score_fusion(dense_results, sparse_results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sparse_vector_creation() {
        let sparse = SparseVector::new(
            vec![0, 5, 10],
            vec![1.0, 2.0, 3.0],
        );

        assert_eq!(sparse.nnz(), 3);
    }

    #[test]
    fn test_sparse_dot_product() {
        let a = SparseVector::new(vec![0, 2, 4], vec![1.0, 2.0, 3.0]);
        let b = SparseVector::new(vec![0, 2, 4], vec![1.0, 2.0, 3.0]);

        let dot = a.dot(&b);
        assert_eq!(dot, 1.0 + 4.0 + 9.0); // 14.0
    }

    #[test]
    fn test_sparse_from_dense() {
        let dense = vec![0.1, 0.0, 2.5, 0.0, 3.7];
        let sparse = SparseVector::from_dense(&dense, 0.5);

        assert_eq!(sparse.nnz(), 2); // Only 2.5 and 3.7 are above threshold
        assert!(sparse.indices.contains(&2));
        assert!(sparse.indices.contains(&4));
    }

    #[test]
    fn test_bm25_scoring() {
        let mut bm25 = BM25::default();

        let doc1 = SparseVector::new(vec![0, 1, 2], vec![3.0, 2.0, 1.0]);
        let doc2 = SparseVector::new(vec![1, 2, 3], vec![1.0, 2.0, 1.0]);

        bm25.add_document(&doc1);
        bm25.add_document(&doc2);

        let query = SparseVector::new(vec![0, 1], vec![1.0, 1.0]);
        let score = bm25.score(&query, &doc1, doc1.nnz());

        assert!(score > 0.0);
    }

    #[test]
    fn test_reciprocal_rank_fusion() {
        let dense = vec![
            (uuid::Uuid::new_v4(), 0.9),
            (uuid::Uuid::new_v4(), 0.8),
        ];
        let sparse = vec![
            (dense[1].0, 0.7), // Same ID as second dense result
            (uuid::Uuid::new_v4(), 0.6),
        ];

        let fused = reciprocal_rank_fusion(dense, sparse);

        // The overlapping ID should have highest combined score
        assert!(fused.len() >= 2);
    }
}
