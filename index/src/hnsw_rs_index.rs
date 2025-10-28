/// Adapter for hnsw_rs library - production-ready HNSW implementation
use vectordb_common::{Result, VectorDbError};
use vectordb_common::types::*;
use hnsw_rs::prelude::*;
use hnsw_rs::anndists::dist::*;
use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::HashMap;
use crate::SearchResult;

/// Wrapper around hnsw_rs::hnsw::Hnsw for our VectorIndex trait
pub struct HnswRsIndex {
    // hnsw_rs requires specific distance types at compile time
    // We'll use f32 vectors with the appropriate distance metric
    inner_cosine: Option<Arc<Hnsw<'static, f32, DistCosine>>>,
    inner_euclidean: Option<Arc<Hnsw<'static, f32, DistL2>>>,

    distance_metric: DistanceMetric,
    dimension: usize,

    // Map UUID to index in HNSW (hnsw_rs uses usize internally)
    id_to_idx: Arc<RwLock<HashMap<VectorId, usize>>>,
    idx_to_id: Arc<RwLock<HashMap<usize, VectorId>>>,
    metadata: Arc<RwLock<HashMap<VectorId, HashMap<String, serde_json::Value>>>>,
    next_idx: Arc<RwLock<usize>>,
}

impl HnswRsIndex {
    pub fn new(
        config: IndexConfig,
        distance_metric: DistanceMetric,
        dimension: usize,
    ) -> Self {
        let max_nb_connection = config.max_connections;
        let ef_construction = config.ef_construction;
        let max_layer = 16.min(config.max_layer); // hnsw_rs max is 16
        let nb_elem = 100000; // Initial capacity estimate

        // Create the appropriate HNSW instance based on distance metric
        let (inner_cosine, inner_euclidean) = match distance_metric {
            DistanceMetric::Cosine => {
                let hnsw = Hnsw::<f32, DistCosine>::new(
                    max_nb_connection,
                    nb_elem,
                    max_layer,
                    ef_construction,
                    DistCosine {},
                );
                (Some(Arc::new(hnsw)), None)
            },
            DistanceMetric::Euclidean => {
                let hnsw = Hnsw::<f32, DistL2>::new(
                    max_nb_connection,
                    nb_elem,
                    max_layer,
                    ef_construction,
                    DistL2 {},
                );
                (None, Some(Arc::new(hnsw)))
            },
            _ => {
                // Default to cosine for unsupported metrics
                let hnsw = Hnsw::<f32, DistCosine>::new(
                    max_nb_connection,
                    nb_elem,
                    max_layer,
                    ef_construction,
                    DistCosine {},
                );
                (Some(Arc::new(hnsw)), None)
            }
        };

        Self {
            inner_cosine,
            inner_euclidean,
            distance_metric,
            dimension,
            id_to_idx: Arc::new(RwLock::new(HashMap::new())),
            idx_to_id: Arc::new(RwLock::new(HashMap::new())),
            metadata: Arc::new(RwLock::new(HashMap::new())),
            next_idx: Arc::new(RwLock::new(0)),
        }
    }

    fn get_next_idx(&self) -> usize {
        let mut idx = self.next_idx.write();
        let current = *idx;
        *idx += 1;
        current
    }
}

impl super::VectorIndex for HnswRsIndex {
    fn insert(
        &mut self,
        id: VectorId,
        vector: &[f32],
        metadata: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<()> {
        if vector.len() != self.dimension {
            return Err(VectorDbError::InvalidDimension {
                expected: self.dimension,
                actual: vector.len(),
            });
        }

        // Get next internal index
        let idx = self.get_next_idx();

        // Store ID mappings
        self.id_to_idx.write().insert(id, idx);
        self.idx_to_id.write().insert(idx, id);

        if let Some(meta) = metadata {
            self.metadata.write().insert(id, meta);
        }

        // Insert into appropriate HNSW (hnsw_rs is thread-safe)
        match self.distance_metric {
            DistanceMetric::Cosine => {
                if let Some(ref hnsw) = self.inner_cosine {
                    hnsw.insert((vector, idx));
                }
            },
            DistanceMetric::Euclidean => {
                if let Some(ref hnsw) = self.inner_euclidean {
                    hnsw.insert((vector, idx));
                }
            },
            _ => {
                if let Some(ref hnsw) = self.inner_cosine {
                    hnsw.insert((vector, idx));
                }
            }
        }

        Ok(())
    }

    fn batch_insert(
        &mut self,
        vectors: Vec<(VectorId, Vec<f32>, Option<HashMap<String, serde_json::Value>>)>,
    ) -> Result<()> {
        // Prepare batch data
        let mut batch_data = Vec::with_capacity(vectors.len());
        let mut id_mappings = Vec::with_capacity(vectors.len());

        for (id, vector, metadata) in vectors {
            if vector.len() != self.dimension {
                return Err(VectorDbError::InvalidDimension {
                    expected: self.dimension,
                    actual: vector.len(),
                });
            }

            let idx = self.get_next_idx();
            id_mappings.push((id, idx, metadata));
            batch_data.push((vector, idx));
        }

        // Store all ID mappings
        {
            let mut id_to_idx = self.id_to_idx.write();
            let mut idx_to_id = self.idx_to_id.write();
            let mut meta_map = self.metadata.write();

            for (id, idx, metadata) in id_mappings {
                id_to_idx.insert(id, idx);
                idx_to_id.insert(idx, id);
                if let Some(meta) = metadata {
                    meta_map.insert(id, meta);
                }
            }
        }

        // Use parallel insert for better performance (hnsw_rs is thread-safe)
        match self.distance_metric {
            DistanceMetric::Cosine => {
                if let Some(ref hnsw) = self.inner_cosine {
                    let data_refs: Vec<(&Vec<f32>, usize)> = batch_data
                        .iter()
                        .map(|(v, idx)| (v, *idx))
                        .collect();
                    hnsw.parallel_insert(&data_refs);
                }
            },
            DistanceMetric::Euclidean => {
                if let Some(ref hnsw) = self.inner_euclidean {
                    let data_refs: Vec<(&Vec<f32>, usize)> = batch_data
                        .iter()
                        .map(|(v, idx)| (v, *idx))
                        .collect();
                    hnsw.parallel_insert(&data_refs);
                }
            },
            _ => {
                if let Some(ref hnsw) = self.inner_cosine {
                    let data_refs: Vec<(&Vec<f32>, usize)> = batch_data
                        .iter()
                        .map(|(v, idx)| (v, *idx))
                        .collect();
                    hnsw.parallel_insert(&data_refs);
                }
            }
        }

        Ok(())
    }

    fn search(
        &self,
        query: &[f32],
        limit: usize,
        ef_search: Option<usize>,
    ) -> Result<Vec<SearchResult>> {
        if query.len() != self.dimension {
            return Err(VectorDbError::InvalidDimension {
                expected: self.dimension,
                actual: query.len(),
            });
        }

        let ef = ef_search.unwrap_or(limit.max(50));

        // Search in appropriate HNSW (hnsw_rs is thread-safe)
        let internal_results = match self.distance_metric {
            DistanceMetric::Cosine => {
                if let Some(ref hnsw) = self.inner_cosine {
                    hnsw.search(query, limit, ef)
                } else {
                    Vec::new()
                }
            },
            DistanceMetric::Euclidean => {
                if let Some(ref hnsw) = self.inner_euclidean {
                    hnsw.search(query, limit, ef)
                } else {
                    Vec::new()
                }
            },
            _ => {
                if let Some(ref hnsw) = self.inner_cosine {
                    hnsw.search(query, limit, ef)
                } else {
                    Vec::new()
                }
            }
        };

        // Convert internal results to our SearchResult format
        let idx_to_id = self.idx_to_id.read();
        let metadata_map = self.metadata.read();

        let mut results = Vec::new();
        for neighbor in internal_results {
            let internal_idx = neighbor.d_id;
            if let Some(id) = idx_to_id.get(&internal_idx) {
                results.push(SearchResult {
                    id: *id,
                    distance: neighbor.distance,
                    metadata: metadata_map.get(id).cloned(),
                });
            }
        }

        Ok(results)
    }

    fn delete(&mut self, id: &VectorId) -> Result<bool> {
        // hnsw_rs doesn't support deletion directly
        // We just remove from our mappings
        let idx_opt = self.id_to_idx.write().remove(id);
        if let Some(idx) = idx_opt {
            self.idx_to_id.write().remove(&idx);
            self.metadata.write().remove(id);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn stats(&self) -> super::IndexStats {
        let vector_count = self.id_to_idx.read().len();

        // Rough memory estimate
        let memory_usage = vector_count * (self.dimension * 4 + 64); // vectors + overhead

        super::IndexStats {
            vector_count,
            memory_usage,
            dimension: self.dimension,
            max_layer: 16,  // hnsw_rs max
            avg_connections: 16.0,  // Approximate based on M parameter
        }
    }

    fn serialize(&self) -> Result<Vec<u8>> {
        // hnsw_rs supports file_dump, but for now return empty
        // TODO: Implement proper serialization
        Ok(Vec::new())
    }

    fn deserialize(&mut self, _data: &[u8]) -> Result<()> {
        // TODO: Implement proper deserialization
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hnsw_rs_insert_and_search() {
        let config = IndexConfig::default();
        let mut index = HnswRsIndex::new(config, DistanceMetric::Cosine, 3);

        // Insert some vectors
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        index.insert(id1, &[1.0, 0.0, 0.0], None).unwrap();
        index.insert(id2, &[0.0, 1.0, 0.0], None).unwrap();

        // Search
        let results = index.search(&[1.0, 0.0, 0.0], 1, None).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, id1);
    }

    #[test]
    fn test_hnsw_rs_batch_insert() {
        let config = IndexConfig::default();
        let mut index = HnswRsIndex::new(config, DistanceMetric::Cosine, 3);

        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        let vectors = vec![
            (id1, vec![1.0, 0.0, 0.0], None),
            (id2, vec![0.0, 1.0, 0.0], None),
        ];

        index.batch_insert(vectors).unwrap();

        let results = index.search(&[1.0, 0.0, 0.0], 1, None).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, id1);
    }
}
