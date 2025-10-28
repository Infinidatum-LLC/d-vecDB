pub mod hnsw;
pub mod node;

use vectordb_common::Result;
use vectordb_common::types::*;

pub use hnsw::*;
pub use node::*;

/// Search result with distance and metadata
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub id: VectorId,
    pub distance: f32,
    pub metadata: Option<std::collections::HashMap<String, serde_json::Value>>,
}

impl PartialEq for SearchResult {
    fn eq(&self, other: &Self) -> bool {
        self.distance == other.distance
    }
}

impl Eq for SearchResult {}

impl PartialOrd for SearchResult {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        // Reverse ordering for min-heap (smaller distances first)
        other.distance.partial_cmp(&self.distance)
    }
}

impl Ord for SearchResult {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap_or(std::cmp::Ordering::Equal)
    }
}

/// Trait for vector index implementations
pub trait VectorIndex: Send + Sync {
    /// Insert a vector into the index
    fn insert(&mut self, id: VectorId, vector: &[f32], metadata: Option<std::collections::HashMap<String, serde_json::Value>>) -> Result<()>;

    /// Batch insert vectors (optimized for performance)
    /// Default implementation falls back to sequential insert
    fn batch_insert(
        &mut self,
        vectors: Vec<(VectorId, Vec<f32>, Option<std::collections::HashMap<String, serde_json::Value>>)>
    ) -> Result<()> {
        // Default fallback: sequential insert
        for (id, vector, metadata) in vectors {
            self.insert(id, &vector, metadata)?;
        }
        Ok(())
    }

    /// Search for nearest neighbors
    fn search(&self, query: &[f32], limit: usize, ef: Option<usize>) -> Result<Vec<SearchResult>>;

    /// Delete a vector from the index
    fn delete(&mut self, id: &VectorId) -> Result<bool>;

    /// Update a vector in the index
    fn update(&mut self, id: VectorId, vector: &[f32], metadata: Option<std::collections::HashMap<String, serde_json::Value>>) -> Result<()> {
        self.delete(&id)?;
        self.insert(id, vector, metadata)?;
        Ok(())
    }

    /// Get index statistics
    fn stats(&self) -> IndexStats;

    /// Serialize index to bytes
    fn serialize(&self) -> Result<Vec<u8>>;

    /// Deserialize index from bytes
    fn deserialize(&mut self, data: &[u8]) -> Result<()>;
}

/// Index statistics
#[derive(Debug, Clone)]
pub struct IndexStats {
    pub vector_count: usize,
    pub memory_usage: usize,
    pub dimension: usize,
    pub max_layer: usize,
    pub avg_connections: f32,
}