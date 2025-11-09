use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::filter::Filter;

/// Recommendation API request - find vectors similar to positive examples
/// and dissimilar to negative examples
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendRequest {
    pub collection: String,
    /// IDs of positive examples (similar to these)
    pub positive: Vec<Uuid>,
    /// IDs of negative examples (dissimilar to these)
    #[serde(default)]
    pub negative: Vec<Uuid>,
    /// Optional filter conditions
    pub filter: Option<Filter>,
    /// Number of results to return
    pub limit: usize,
    /// Search strategy
    #[serde(default)]
    pub strategy: RecommendStrategy,
    /// Offset for pagination
    #[serde(default)]
    pub offset: usize,
}

/// Strategy for combining positive and negative examples
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecommendStrategy {
    /// Average vectors and search (default)
    AverageVector,
    /// Best score across all positive examples
    BestScore,
}

impl Default for RecommendStrategy {
    fn default() -> Self {
        Self::AverageVector
    }
}

/// Discovery API request - find vectors that lie "between" positive and negative examples
/// Useful for exploration and discovering new relevant content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryRequest {
    pub collection: String,
    /// Target vector or ID to start from
    pub target: DiscoveryTarget,
    /// Context pairs (positive, negative) to define the search direction
    pub context: Vec<ContextPair>,
    /// Optional filter conditions
    pub filter: Option<Filter>,
    /// Number of results to return
    pub limit: usize,
    /// Offset for pagination
    #[serde(default)]
    pub offset: usize,
}

/// Discovery target (starting point)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DiscoveryTarget {
    VectorId(Uuid),
    Vector(Vec<f32>),
}

/// Context pair for discovery search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPair {
    /// Positive example (move towards this)
    pub positive: Uuid,
    /// Negative example (move away from this)
    pub negative: Uuid,
}

/// Scroll request - paginate through all vectors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrollRequest {
    pub collection: String,
    /// Optional filter
    pub filter: Option<Filter>,
    /// Number of results per page
    pub limit: usize,
    /// Scroll offset or cursor
    pub offset: Option<String>,
    /// Whether to include vectors in response
    #[serde(default = "default_true")]
    pub with_vectors: bool,
    /// Whether to include payload/metadata
    #[serde(default = "default_true")]
    pub with_payload: bool,
}

fn default_true() -> bool {
    true
}

/// Scroll response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrollResponse {
    pub points: Vec<ScoredPoint>,
    /// Next scroll offset (None if end reached)
    pub next_offset: Option<String>,
}

/// Scored point in results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredPoint {
    pub id: Uuid,
    pub score: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vector: Option<Vec<f32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<std::collections::HashMap<String, serde_json::Value>>,
}

/// Count request - count vectors matching a filter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountRequest {
    pub collection: String,
    /// Filter condition
    pub filter: Option<Filter>,
    /// Whether to return exact count (slower) or estimate
    #[serde(default)]
    pub exact: bool,
}

/// Count response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountResponse {
    pub count: usize,
}

/// Batch search request - multiple queries in one request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchSearchRequest {
    pub collection: String,
    pub searches: Vec<SearchQuery>,
}

/// Individual search query in batch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub vector: Vec<f32>,
    pub filter: Option<Filter>,
    pub limit: usize,
    #[serde(default)]
    pub offset: usize,
}

/// Query with prefetch - first filter with approximate search, then refine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryWithPrefetch {
    pub collection: String,
    /// Initial prefetch queries (approximate, quantized)
    pub prefetch: Vec<PrefetchQuery>,
    /// Final query on prefetched results
    pub query: SearchQuery,
}

/// Prefetch query (approximate search)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefetchQuery {
    pub vector: Vec<f32>,
    pub filter: Option<Filter>,
    /// Use quantized index for faster prefetch
    #[serde(default)]
    pub using_quantization: bool,
    pub limit: usize,
}

/// Group by request - group results by payload field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupByRequest {
    pub collection: String,
    pub vector: Vec<f32>,
    /// Field to group by
    pub group_by: String,
    /// Results per group
    pub group_size: usize,
    /// Number of groups
    pub limit: usize,
    pub filter: Option<Filter>,
}

/// Facet request - get distribution of values for a field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FacetRequest {
    pub collection: String,
    /// Field to facet on
    pub field: String,
    /// Optional filter
    pub filter: Option<Filter>,
    /// Maximum number of facet values
    #[serde(default = "default_facet_limit")]
    pub limit: usize,
}

fn default_facet_limit() -> usize {
    10
}

/// Facet response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FacetResponse {
    pub field: String,
    pub values: Vec<FacetValue>,
}

/// Individual facet value with count
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FacetValue {
    pub value: serde_json::Value,
    pub count: usize,
}

// Helper functions for recommendation search

/// Compute average vector from multiple vectors
pub fn average_vectors(vectors: &[Vec<f32>]) -> Option<Vec<f32>> {
    if vectors.is_empty() {
        return None;
    }

    let dim = vectors[0].len();
    let mut avg = vec![0.0; dim];

    for vector in vectors {
        if vector.len() != dim {
            return None; // Dimension mismatch
        }
        for (i, &v) in vector.iter().enumerate() {
            avg[i] += v;
        }
    }

    let count = vectors.len() as f32;
    for v in &mut avg {
        *v /= count;
    }

    Some(avg)
}

/// Compute recommendation vector from positive and negative examples
pub fn compute_recommendation_vector(
    positive_vectors: &[Vec<f32>],
    negative_vectors: &[Vec<f32>],
) -> Option<Vec<f32>> {
    let positive_avg = average_vectors(positive_vectors)?;

    if negative_vectors.is_empty() {
        return Some(positive_avg);
    }

    let negative_avg = average_vectors(negative_vectors)?;

    if positive_avg.len() != negative_avg.len() {
        return None;
    }

    // Move away from negative: positive + (positive - negative)
    let mut result = Vec::with_capacity(positive_avg.len());
    for (p, n) in positive_avg.iter().zip(negative_avg.iter()) {
        result.push(2.0 * p - n);
    }

    Some(result)
}

/// Compute discovery direction from context pairs
pub fn compute_discovery_direction(
    target: &[f32],
    context_vectors: &[(Vec<f32>, Vec<f32>)], // (positive, negative) pairs
) -> Option<Vec<f32>> {
    if context_vectors.is_empty() {
        return Some(target.to_vec());
    }

    let dim = target.len();
    let mut direction = vec![0.0; dim];

    // Compute average direction from all context pairs
    for (positive, negative) in context_vectors {
        if positive.len() != dim || negative.len() != dim {
            return None;
        }

        for i in 0..dim {
            direction[i] += positive[i] - negative[i];
        }
    }

    // Normalize direction
    let count = context_vectors.len() as f32;
    for v in &mut direction {
        *v /= count;
    }

    // Apply direction to target
    let mut result = Vec::with_capacity(dim);
    for (t, d) in target.iter().zip(direction.iter()) {
        result.push(t + d);
    }

    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_average_vectors() {
        let vectors = vec![
            vec![1.0, 2.0, 3.0],
            vec![3.0, 4.0, 5.0],
        ];

        let avg = average_vectors(&vectors).unwrap();
        assert_eq!(avg, vec![2.0, 3.0, 4.0]);
    }

    #[test]
    fn test_recommendation_vector() {
        let positive = vec![vec![1.0, 1.0]];
        let negative = vec![vec![-1.0, -1.0]];

        let rec = compute_recommendation_vector(&positive, &negative).unwrap();

        // Should move away from negative: 2*positive - negative
        assert_eq!(rec, vec![3.0, 3.0]);
    }

    #[test]
    fn test_discovery_direction() {
        let target = vec![0.0, 0.0];
        let context = vec![
            (vec![1.0, 0.0], vec![-1.0, 0.0]),
            (vec![0.0, 1.0], vec![0.0, -1.0]),
        ];

        let result = compute_discovery_direction(&target, &context).unwrap();

        // Average direction: [(2, 0), (0, 2)] / 2 = (1, 1)
        // Applied to target (0, 0): result = (1, 1)
        assert_eq!(result, vec![1.0, 1.0]);
    }
}
