use vectordb_common::{Result, VectorDbError};
use vectordb_common::types::*;
use vectordb_storage::StorageEngine;
use vectordb_index::{VectorIndex, HnswRsIndex};  // Use production-ready HNSW
use std::sync::Arc;
use dashmap::DashMap;
use tracing::{info, error};
use metrics::{counter, histogram, gauge};

/// Main vector store engine that coordinates storage and indexing
pub struct VectorStore {
    storage: StorageEngine,
    indexes: Arc<DashMap<CollectionId, Box<dyn VectorIndex>>>,
}

impl VectorStore {
    /// Create a new vector store
    pub async fn new<P: AsRef<std::path::Path>>(data_dir: P) -> Result<Self> {
        let storage = StorageEngine::new(data_dir).await?;

        let mut store = Self {
            storage,
            indexes: Arc::new(DashMap::new()),
        };

        // Rebuild indexes for existing collections
        store.rebuild_indexes().await?;

        Ok(store)
    }
    
    /// Create a new collection
    pub async fn create_collection(&self, config: &CollectionConfig) -> Result<()> {
        info!("Creating collection: {}", config.name);
        counter!("vectorstore.collections.created").increment(1);
        
        // Create storage
        self.storage.create_collection(config).await?;

        // Create index - using production-ready hnsw_rs
        let index = Box::new(HnswRsIndex::new(
            config.index_config.clone(),
            config.distance_metric,
            config.dimension,
        ));

        self.indexes.insert(config.name.clone(), index);
        
        info!("Collection created successfully: {}", config.name);
        Ok(())
    }
    
    /// Delete a collection with soft-delete (recoverable for 24 hours)
    pub async fn delete_collection(&self, name: &str) -> Result<()> {
        info!("Soft-deleting collection: {}", name);
        counter!("vectorstore.collections.deleted").increment(1);

        // Use soft delete via recovery manager
        let recovery = self.storage.get_recovery_manager();
        recovery.soft_delete_collection(name).await?;
        self.indexes.remove(name);

        info!("Collection soft-deleted successfully: {} (recoverable for 24 hours)", name);
        Ok(())
    }

    /// Hard delete a collection (permanent, no recovery)
    pub async fn hard_delete_collection(&self, name: &str) -> Result<()> {
        info!("Permanently deleting collection: {}", name);
        counter!("vectorstore.collections.hard_deleted").increment(1);

        self.storage.delete_collection(name).await?;
        self.indexes.remove(name);

        info!("Collection permanently deleted: {}", name);
        Ok(())
    }

    /// Restore a soft-deleted or backed-up collection
    pub async fn restore_collection(&self, backup_path: &std::path::Path, collection_name: Option<&str>) -> Result<String> {
        info!("Restoring collection from: {}", backup_path.display());

        let recovery = self.storage.get_recovery_manager();
        let name = recovery.restore_collection(backup_path, collection_name).await?;

        // Rebuild index for restored collection
        if let Some(config) = self.storage.get_collection_config(&name)? {
            let index = Box::new(HnswRsIndex::new(
                config.index_config.clone(),
                config.distance_metric,
                config.dimension,
            ));
            self.indexes.insert(name.clone(), index);
        }

        info!("Collection restored successfully: {}", name);
        Ok(name)
    }

    /// Import orphaned collection data (vectors.bin/index.bin files)
    pub async fn import_orphaned_collection(&self, orphaned_dir: &std::path::Path, new_collection_name: &str, config: &CollectionConfig) -> Result<()> {
        info!("Importing orphaned collection from: {}", orphaned_dir.display());

        let recovery = self.storage.get_recovery_manager();
        recovery.import_orphaned_collection(orphaned_dir, new_collection_name).await?;

        // Register the collection with storage engine
        self.storage.register_imported_collection(config).await?;

        // Create index for imported collection
        let index = Box::new(HnswRsIndex::new(
            config.index_config.clone(),
            config.distance_metric,
            config.dimension,
        ));
        self.indexes.insert(config.name.clone(), index);

        info!("Orphaned collection imported successfully as: {}", new_collection_name);
        Ok(())
    }

    /// List all soft-deleted collections
    pub async fn list_deleted_collections(&self) -> Result<Vec<(String, std::path::PathBuf, u64)>> {
        let recovery = self.storage.get_recovery_manager();
        recovery.list_deleted_collections().await
    }

    /// Cleanup old soft-deleted collections (older than retention_hours)
    pub async fn cleanup_old_deleted(&self, retention_hours: u64) -> Result<Vec<String>> {
        let recovery = self.storage.get_recovery_manager();
        recovery.cleanup_old_deleted(retention_hours).await
    }

    /// Create backup of a specific collection
    pub async fn backup_collection(&self, collection_name: &str) -> Result<std::path::PathBuf> {
        let recovery = self.storage.get_recovery_manager();
        recovery.backup_collection(collection_name).await
    }
    
    /// Insert a vector into a collection
    pub async fn insert(&self, collection: &str, vector: &Vector) -> Result<()> {
        let start = std::time::Instant::now();
        counter!("vectorstore.vectors.inserted").increment(1);
        
        // Validate collection exists
        let config = self.get_collection_config(collection)?
            .ok_or_else(|| VectorDbError::CollectionNotFound {
                name: collection.to_string(),
            })?;
        
        // Validate vector dimension
        if vector.data.len() != config.dimension {
            return Err(VectorDbError::InvalidDimension {
                expected: config.dimension,
                actual: vector.data.len(),
            });
        }
        
        // Insert into storage
        self.storage.insert_vector(collection, vector).await?;

        // OPTIMIZATION: Direct insert without spawn_blocking overhead
        // hnsw_rs is thread-safe, DashMap provides lock-free access
        if let Some(mut index) = self.indexes.get_mut(collection) {
            index.insert(vector.id, &vector.data, vector.metadata.clone())?;
        }

        histogram!("vectorstore.insert.duration").record(start.elapsed().as_secs_f64());
        Ok(())
    }
    
    /// Batch insert vectors
    pub async fn batch_insert(&self, collection: &str, vectors: &[Vector]) -> Result<()> {
        let start = std::time::Instant::now();
        counter!("vectorstore.vectors.batch_inserted").increment(vectors.len() as u64);

        if vectors.is_empty() {
            return Ok(());
        }

        // Validate collection exists
        let config = self.get_collection_config(collection)?
            .ok_or_else(|| VectorDbError::CollectionNotFound {
                name: collection.to_string(),
            })?;

        // Validate all vector dimensions
        for vector in vectors {
            if vector.data.len() != config.dimension {
                return Err(VectorDbError::InvalidDimension {
                    expected: config.dimension,
                    actual: vector.data.len(),
                });
            }
        }

        // Insert into storage (async operation)
        self.storage.batch_insert(collection, vectors).await?;

        // OPTIMIZATION: Direct batch insert without spawn_blocking overhead
        // hnsw_rs is already thread-safe with internal parallel processing
        // Avoiding spawn_blocking + cloning saves significant overhead

        // Prepare vectors for batch insert (zero-copy where possible)
        let vectors_to_insert: Vec<(VectorId, Vec<f32>, Option<_>)> = vectors.iter()
            .map(|v| (v.id, v.data.clone(), v.metadata.clone()))
            .collect();

        // Direct call to batch_insert - DashMap provides lock-free access
        // hnsw_rs::parallel_insert uses rayon internally for multi-threading
        if let Some(mut index) = self.indexes.get_mut(collection) {
            index.batch_insert(vectors_to_insert)?;
        }

        histogram!("vectorstore.batch_insert.duration").record(start.elapsed().as_secs_f64());
        info!("Batch inserted {} vectors into {}", vectors.len(), collection);
        Ok(())
    }
    
    /// Query vectors for nearest neighbors
    pub async fn query(&self, request: &QueryRequest) -> Result<Vec<QueryResult>> {
        let start = std::time::Instant::now();
        counter!("vectorstore.queries").increment(1);

        // Validate collection exists
        let config = self.get_collection_config(&request.collection)?
            .ok_or_else(|| VectorDbError::CollectionNotFound {
                name: request.collection.clone(),
            })?;

        // Validate query vector dimension
        if request.vector.len() != config.dimension {
            return Err(VectorDbError::InvalidDimension {
                expected: config.dimension,
                actual: request.vector.len(),
            });
        }

        // Search index - DashMap provides lock-free reads
        let index = self.indexes
            .get(&request.collection)
            .ok_or_else(|| VectorDbError::CollectionNotFound {
                name: request.collection.clone(),
            })?;

        // If filter is present, search with larger candidate pool for post-filtering
        let search_limit = if request.filter.is_some() {
            // Search more candidates to account for filtering
            request.limit * 3
        } else {
            request.limit
        };

        let search_results = index.search(&request.vector, search_limit, request.ef_search)?;

        // Apply payload filter if present
        let filtered_results: Vec<vectordb_index::SearchResult> = if let Some(filter) = &request.filter {
            search_results
                .into_iter()
                .filter(|r| vectordb_common::filter::evaluate_filter(filter, &r.metadata))
                .take(request.limit)
                .collect()
        } else {
            search_results.into_iter().take(request.limit).collect()
        };

        // Convert to QueryResult
        let results: Vec<QueryResult> = filtered_results
            .into_iter()
            .map(|r| QueryResult {
                id: r.id,
                distance: r.distance,
                metadata: r.metadata,
            })
            .collect();

        histogram!("vectorstore.query.duration").record(start.elapsed().as_secs_f64());
        histogram!("vectorstore.query.results").record(results.len() as f64);
        Ok(results)
    }

    /// Recommend vectors based on positive and negative examples
    pub async fn recommend(&self, request: &vectordb_common::RecommendRequest) -> Result<Vec<QueryResult>> {
        let start = std::time::Instant::now();
        counter!("vectorstore.recommend").increment(1);

        // Get positive example vectors
        let mut positive_vectors = Vec::new();
        for id in &request.positive {
            if let Some(vector) = self.get(&request.collection, id).await? {
                positive_vectors.push(vector.data);
            }
        }

        if positive_vectors.is_empty() {
            return Err(VectorDbError::NotFound {
                message: "No positive examples found".to_string(),
            });
        }

        // Get negative example vectors
        let mut negative_vectors = Vec::new();
        for id in &request.negative {
            if let Some(vector) = self.get(&request.collection, id).await? {
                negative_vectors.push(vector.data);
            }
        }

        // Compute recommendation vector
        let query_vector = vectordb_common::compute_recommendation_vector(
            &positive_vectors,
            &negative_vectors,
        )
        .ok_or_else(|| VectorDbError::InvalidInput {
            message: "Failed to compute recommendation vector".to_string(),
        })?;

        // Execute search with computed vector
        let query_request = QueryRequest {
            collection: request.collection.clone(),
            vector: query_vector,
            limit: request.limit + request.offset,
            ef_search: None,
            filter: request.filter.clone(),
        };

        let mut results = self.query(&query_request).await?;

        // Apply offset and limit
        if request.offset > 0 {
            results = results.into_iter().skip(request.offset).collect();
        }
        results.truncate(request.limit);

        histogram!("vectorstore.recommend.duration").record(start.elapsed().as_secs_f64());
        Ok(results)
    }

    /// Discovery search - find vectors in the direction of positive/negative context
    pub async fn discover(&self, request: &vectordb_common::DiscoveryRequest) -> Result<Vec<QueryResult>> {
        let start = std::time::Instant::now();
        counter!("vectorstore.discover").increment(1);

        // Get target vector
        let target_vector = match &request.target {
            vectordb_common::DiscoveryTarget::VectorId(id) => {
                let vector = self.get(&request.collection, id).await?
                    .ok_or_else(|| VectorDbError::NotFound {
                        message: format!("Target vector not found: {}", id),
                    })?;
                vector.data
            }
            vectordb_common::DiscoveryTarget::Vector(v) => v.clone(),
        };

        // Get context pairs
        let mut context_vectors = Vec::new();
        for pair in &request.context {
            let positive = self.get(&request.collection, &pair.positive).await?
                .ok_or_else(|| VectorDbError::NotFound {
                    message: format!("Positive vector not found: {}", pair.positive),
                })?;

            let negative = self.get(&request.collection, &pair.negative).await?
                .ok_or_else(|| VectorDbError::NotFound {
                    message: format!("Negative vector not found: {}", pair.negative),
                })?;

            context_vectors.push((positive.data, negative.data));
        }

        // Compute discovery direction
        let query_vector = vectordb_common::compute_discovery_direction(
            &target_vector,
            &context_vectors,
        )
        .ok_or_else(|| VectorDbError::InvalidInput {
            message: "Failed to compute discovery direction".to_string(),
        })?;

        // Execute search
        let query_request = QueryRequest {
            collection: request.collection.clone(),
            vector: query_vector,
            limit: request.limit + request.offset,
            ef_search: None,
            filter: request.filter.clone(),
        };

        let mut results = self.query(&query_request).await?;

        // Apply offset
        if request.offset > 0 {
            results = results.into_iter().skip(request.offset).collect();
        }
        results.truncate(request.limit);

        histogram!("vectorstore.discover.duration").record(start.elapsed().as_secs_f64());
        Ok(results)
    }

    /// Scroll through all vectors in a collection
    pub async fn scroll(&self, request: &vectordb_common::ScrollRequest) -> Result<vectordb_common::ScrollResponse> {
        let start = std::time::Instant::now();
        counter!("vectorstore.scroll").increment(1);

        // Parse offset (simple index-based for now)
        let offset = request.offset.as_ref()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);

        // Get all vectors from storage
        let all_vectors = self.storage.get_all_vectors(&request.collection).await?;

        // Apply filter if present
        let filtered_vectors: Vec<_> = if let Some(filter) = &request.filter {
            all_vectors.into_iter()
                .filter(|v| vectordb_common::filter::evaluate_filter(filter, &v.metadata))
                .collect()
        } else {
            all_vectors
        };

        // Apply pagination
        let total = filtered_vectors.len();
        let end = (offset + request.limit).min(total);
        let page_vectors: Vec<_> = filtered_vectors.into_iter()
            .skip(offset)
            .take(request.limit)
            .collect();

        // Convert to ScoredPoint
        let points: Vec<vectordb_common::ScoredPoint> = page_vectors.into_iter()
            .map(|v| vectordb_common::ScoredPoint {
                id: v.id,
                score: 0.0, // Scroll doesn't have scores
                vector: if request.with_vectors { Some(v.data) } else { None },
                payload: if request.with_payload { v.metadata } else { None },
            })
            .collect();

        // Compute next offset
        let next_offset = if end < total {
            Some(end.to_string())
        } else {
            None
        };

        histogram!("vectorstore.scroll.duration").record(start.elapsed().as_secs_f64());

        Ok(vectordb_common::ScrollResponse {
            points,
            next_offset,
        })
    }

    /// Count vectors matching a filter
    pub async fn count(&self, request: &vectordb_common::CountRequest) -> Result<vectordb_common::CountResponse> {
        let start = std::time::Instant::now();
        counter!("vectorstore.count").increment(1);

        let count = if let Some(filter) = &request.filter {
            // Get all vectors and filter
            let all_vectors = self.storage.get_all_vectors(&request.collection).await?;
            all_vectors.into_iter()
                .filter(|v| vectordb_common::filter::evaluate_filter(filter, &v.metadata))
                .count()
        } else {
            // No filter - just get total count from stats
            self.get_collection_stats(&request.collection).await?
                .map(|s| s.vector_count)
                .unwrap_or(0)
        };

        histogram!("vectorstore.count.duration").record(start.elapsed().as_secs_f64());

        Ok(vectordb_common::CountResponse { count })
    }

    /// Batch search - multiple queries in one request
    pub async fn batch_search(&self, request: &vectordb_common::BatchSearchRequest) -> Result<Vec<Vec<QueryResult>>> {
        let start = std::time::Instant::now();
        counter!("vectorstore.batch_search").increment(1);

        let mut all_results = Vec::new();

        for search in &request.searches {
            let query_request = QueryRequest {
                collection: request.collection.clone(),
                vector: search.vector.clone(),
                limit: search.limit + search.offset,
                ef_search: None,
                filter: search.filter.clone(),
            };

            let mut results = self.query(&query_request).await?;

            // Apply offset
            if search.offset > 0 {
                results = results.into_iter().skip(search.offset).collect();
            }
            results.truncate(search.limit);

            all_results.push(results);
        }

        histogram!("vectorstore.batch_search.duration").record(start.elapsed().as_secs_f64());
        Ok(all_results)
    }

    /// Delete a vector
    pub async fn delete(&self, collection: &str, id: &VectorId) -> Result<bool> {
        counter!("vectorstore.vectors.deleted").increment(1);
        
        // Delete from storage
        let storage_deleted = self.storage.delete_vector(collection, id).await?;

        // Delete from index - DashMap provides lock-free access
        if let Some(mut index) = self.indexes.get_mut(collection) {
            index.delete(id)?;
        }

        Ok(storage_deleted)
    }
    
    /// Update a vector
    pub async fn update(&self, collection: &str, vector: &Vector) -> Result<()> {
        counter!("vectorstore.vectors.updated").increment(1);
        
        // For now, implement as delete + insert
        // A more efficient implementation would update in-place
        self.delete(collection, &vector.id).await?;
        self.insert(collection, vector).await?;
        
        Ok(())
    }
    
    /// Get a vector by ID
    pub async fn get(&self, collection: &str, id: &VectorId) -> Result<Option<Vector>> {
        self.storage.get_vector(collection, id).await
    }
    
    /// List all collections
    pub fn list_collections(&self) -> Vec<CollectionId> {
        self.storage.list_collections()
    }
    
    /// Get collection configuration
    pub fn get_collection_config(&self, name: &str) -> Result<Option<CollectionConfig>> {
        self.storage.get_collection_config(name)
    }
    
    /// Get collection statistics
    pub async fn get_collection_stats(&self, name: &str) -> Result<Option<CollectionStats>> {
        let mut stats = self.storage.get_collection_stats(name).await?;

        if let Some(ref mut stats) = stats {
            // Add index statistics - DashMap provides lock-free reads
            if let Some(index) = self.indexes.get(name) {
                let index_stats = index.stats();
                stats.vector_count = index_stats.vector_count;
                stats.memory_usage += index_stats.memory_usage;
            }
        }

        Ok(stats)
    }
    
    /// Sync all data to disk
    pub async fn sync(&self) -> Result<()> {
        self.storage.sync().await
    }
    
    /// Rebuild indexes from storage (used during startup)
    async fn rebuild_indexes(&mut self) -> Result<()> {
        info!("Rebuilding indexes from storage...");

        let collections = self.storage.list_collections();

        for collection_name in collections {
            if let Some(config) = self.storage.get_collection_config(&collection_name)? {
                info!("Rebuilding index for collection: {}", collection_name);

                // Create new index
                let mut index = Box::new(HnswRsIndex::new(
                    config.index_config.clone(),
                    config.distance_metric,
                    config.dimension,
                ));

                // Load all vectors from storage and rebuild the index
                match self.storage.get_all_vectors(&collection_name).await {
                    Ok(vectors) => {
                        info!("Loading {} vectors into index for collection '{}'", vectors.len(), collection_name);

                        // Prepare vectors for batch insert
                        let vectors_to_insert: Vec<(uuid::Uuid, Vec<f32>, Option<_>)> = vectors
                            .iter()
                            .map(|v| (v.id, v.data.clone(), v.metadata.clone()))
                            .collect();

                        // Batch insert all vectors into the index
                        if !vectors_to_insert.is_empty() {
                            if let Err(e) = index.batch_insert(vectors_to_insert) {
                                error!("Failed to rebuild index for collection '{}': {}", collection_name, e);
                                // Continue with empty index rather than failing completely
                            } else {
                                info!("Successfully rebuilt index for collection '{}' with {} vectors",
                                      collection_name, vectors.len());
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to load vectors for collection '{}': {}", collection_name, e);
                        // Continue with empty index rather than failing completely
                    }
                }

                self.indexes.insert(collection_name.clone(), index);
            }
        }

        info!("Index rebuild completed");
        Ok(())
    }
    
    /// Get server statistics
    pub async fn get_server_stats(&self) -> Result<ServerStats> {
        let collections = self.list_collections();
        let total_collections = collections.len() as u32;
        
        let mut total_vectors = 0u64;
        let mut memory_usage = 0u64;
        
        for collection in &collections {
            if let Some(stats) = self.get_collection_stats(collection).await? {
                total_vectors += stats.vector_count as u64;
                memory_usage += stats.memory_usage as u64;
            }
        }
        
        // Update metrics
        gauge!("vectorstore.collections.total").set(total_collections as f64);
        gauge!("vectorstore.vectors.total").set(total_vectors as f64);
        gauge!("vectorstore.memory.usage").set(memory_usage as f64);
        
        Ok(ServerStats {
            total_vectors,
            total_collections,
            memory_usage,
            disk_usage: 0, // TODO: Calculate actual disk usage
            uptime_seconds: 0, // TODO: Track server uptime
        })
    }

    /// Get snapshot manager
    pub fn get_snapshot_manager(&self) -> Result<vectordb_storage::SnapshotManager> {
        let data_dir = self.storage.get_data_dir();
        vectordb_storage::SnapshotManager::new(data_dir)
            .map_err(|e| VectorDbError::StorageError {
                message: format!("Failed to create snapshot manager: {}", e),
            })
    }

    /// Create a snapshot of a collection
    pub async fn create_snapshot(&self, collection_name: &str) -> Result<vectordb_storage::SnapshotMetadata> {
        info!("Creating snapshot for collection: {}", collection_name);

        // Verify collection exists
        self.get_collection_config(collection_name)?
            .ok_or_else(|| VectorDbError::CollectionNotFound {
                name: collection_name.to_string(),
            })?;

        let snapshot_manager = self.get_snapshot_manager()?;
        let collection_dir = self.storage.get_collection_dir(collection_name)?;

        snapshot_manager.create_snapshot(collection_name, &collection_dir).await
    }

    /// List all snapshots
    pub fn list_snapshots(&self) -> Result<Vec<vectordb_storage::SnapshotMetadata>> {
        let snapshot_manager = self.get_snapshot_manager()?;
        snapshot_manager.list_snapshots()
    }

    /// Get snapshot by name
    pub fn get_snapshot(&self, snapshot_name: &str) -> Result<vectordb_storage::SnapshotMetadata> {
        let snapshot_manager = self.get_snapshot_manager()?;
        snapshot_manager.get_snapshot(snapshot_name)
    }

    /// Delete a snapshot
    pub fn delete_snapshot(&self, snapshot_name: &str) -> Result<()> {
        let snapshot_manager = self.get_snapshot_manager()?;
        snapshot_manager.delete_snapshot(snapshot_name)
    }

    /// Restore collection from snapshot
    pub async fn restore_snapshot(&self, snapshot_name: &str, target_collection: Option<&str>) -> Result<String> {
        info!("Restoring snapshot: {}", snapshot_name);

        let snapshot_manager = self.get_snapshot_manager()?;
        let snapshot = snapshot_manager.get_snapshot(snapshot_name)?;

        let collection_name = target_collection.unwrap_or(&snapshot.collection);
        let target_dir = self.storage.get_collection_dir(collection_name)?;

        snapshot_manager.restore_snapshot(snapshot_name, &target_dir).await?;

        // Rebuild index for restored collection
        if let Some(config) = self.storage.get_collection_config(collection_name)? {
            let index = Box::new(vectordb_index::HnswRsIndex::new(
                config.index_config.clone(),
                config.distance_metric,
                config.dimension,
            ));
            self.indexes.insert(collection_name.to_string(), index);

            // Load vectors and rebuild index
            if let Ok(vectors) = self.storage.get_all_vectors(collection_name).await {
                if let Some(mut index) = self.indexes.get_mut(collection_name) {
                    let vectors_to_insert: Vec<(uuid::Uuid, Vec<f32>, Option<_>)> = vectors
                        .iter()
                        .map(|v| (v.id, v.data.clone(), v.metadata.clone()))
                        .collect();

                    if !vectors_to_insert.is_empty() {
                        let _ = index.batch_insert(vectors_to_insert);
                    }
                }
            }
        }

        info!("Snapshot restored successfully: {}", collection_name);
        Ok(collection_name.to_string())
    }
}

/// Server statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct ServerStats {
    pub total_vectors: u64,
    pub total_collections: u32,
    pub memory_usage: u64,
    pub disk_usage: u64,
    pub uptime_seconds: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    async fn create_test_store() -> VectorStore {
        let temp_dir = tempdir().unwrap();
        VectorStore::new(temp_dir.path()).await.unwrap()
    }
    
    #[tokio::test]
    async fn test_create_collection() {
        let store = create_test_store().await;
        
        let config = CollectionConfig {
            name: "test".to_string(),
            dimension: 128,
            distance_metric: DistanceMetric::Cosine,
            vector_type: VectorType::Float32,
            index_config: IndexConfig::default(),
        };
        
        store.create_collection(&config).await.unwrap();
        
        let collections = store.list_collections();
        assert!(collections.contains(&"test".to_string()));
    }
    
    #[tokio::test]
    async fn test_insert_and_query() {
        let store = create_test_store().await;
        
        let config = CollectionConfig {
            name: "test".to_string(),
            dimension: 3,
            distance_metric: DistanceMetric::Cosine,
            vector_type: VectorType::Float32,
            index_config: IndexConfig::default(),
        };
        
        store.create_collection(&config).await.unwrap();
        
        let vector = Vector {
            id: Uuid::new_v4(),
            data: vec![1.0, 0.0, 0.0],
            metadata: None,
        };
        
        store.insert("test", &vector).await.unwrap();
        
        let query = QueryRequest {
            collection: "test".to_string(),
            vector: vec![1.0, 0.0, 0.0],
            limit: 1,
            ef_search: None,
            filter: None,
        };
        
        let results = store.query(&query).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, vector.id);
    }
}