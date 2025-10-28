use vectordb_common::{Result, VectorDbError};
use vectordb_common::types::*;
use vectordb_storage::StorageEngine;
use vectordb_index::{VectorIndex, HnswIndex};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use tracing::info;
use metrics::{counter, histogram, gauge};

/// Main vector store engine that coordinates storage and indexing
pub struct VectorStore {
    storage: StorageEngine,
    indexes: Arc<RwLock<HashMap<CollectionId, Box<dyn VectorIndex>>>>,
}

impl VectorStore {
    /// Create a new vector store
    pub async fn new<P: AsRef<std::path::Path>>(data_dir: P) -> Result<Self> {
        let storage = StorageEngine::new(data_dir).await?;

        let mut store = Self {
            storage,
            indexes: Arc::new(RwLock::new(HashMap::new())),
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
        
        // Create index
        let index = Box::new(HnswIndex::new(
            config.index_config.clone(),
            config.distance_metric,
            config.dimension,
        ));
        
        self.indexes.write().insert(config.name.clone(), index);
        
        info!("Collection created successfully: {}", config.name);
        Ok(())
    }
    
    /// Delete a collection
    pub async fn delete_collection(&self, name: &str) -> Result<()> {
        info!("Deleting collection: {}", name);
        counter!("vectorstore.collections.deleted").increment(1);
        
        self.storage.delete_collection(name).await?;
        self.indexes.write().remove(name);
        
        info!("Collection deleted successfully: {}", name);
        Ok(())
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

        // Insert into index - Move expensive HNSW insert to blocking thread pool
        // This prevents the write lock from blocking the async runtime
        let collection_name = collection.to_string();
        let vector_id = vector.id.clone();
        let vector_data = vector.data.clone();
        let vector_metadata = vector.metadata.clone();

        // Clone Arc to move into blocking task
        let indexes = Arc::clone(&self.indexes);

        // Spawn blocking task to avoid holding write lock in async context
        tokio::task::spawn_blocking(move || {
            let mut indexes_guard = indexes.write();
            if let Some(index) = indexes_guard.get_mut(&collection_name) {
                index.insert(vector_id, &vector_data, vector_metadata)?;
            }
            Ok::<(), VectorDbError>(())
        })
        .await
        .map_err(|e| VectorDbError::Internal {
            message: format!("Failed to spawn blocking task: {}", e)
        })??;

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
        
        // Insert into storage
        self.storage.batch_insert(collection, vectors).await?;

        // Insert into index - Move expensive HNSW batch insert to blocking thread pool
        // This is critical for batch operations as they hold the lock even longer
        let collection_name = collection.to_string();
        // Clone vector data to move into blocking task
        let vectors_to_insert: Vec<Vector> = vectors.to_vec();

        // Clone Arc to move into blocking task
        let indexes = Arc::clone(&self.indexes);

        // Spawn blocking task for batch insert
        tokio::task::spawn_blocking(move || {
            let mut indexes_guard = indexes.write();
            if let Some(index) = indexes_guard.get_mut(&collection_name) {
                for vector in vectors_to_insert {
                    index.insert(vector.id, &vector.data, vector.metadata)?;
                }
            }
            Ok::<(), VectorDbError>(())
        })
        .await
        .map_err(|e| VectorDbError::Internal {
            message: format!("Failed to spawn blocking task for batch insert: {}", e)
        })??;

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
        
        // Search index
        let indexes = self.indexes.read();
        let index = indexes
            .get(&request.collection)
            .ok_or_else(|| VectorDbError::CollectionNotFound {
                name: request.collection.clone(),
            })?;
        
        let search_results = index.search(&request.vector, request.limit, request.ef_search)?;
        
        // Convert to QueryResult
        let results: Vec<QueryResult> = search_results
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
    
    /// Delete a vector
    pub async fn delete(&self, collection: &str, id: &VectorId) -> Result<bool> {
        counter!("vectorstore.vectors.deleted").increment(1);
        
        // Delete from storage
        let storage_deleted = self.storage.delete_vector(collection, id).await?;
        
        // Delete from index
        let mut indexes = self.indexes.write();
        if let Some(index) = indexes.get_mut(collection) {
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
            // Add index statistics
            let indexes = self.indexes.read();
            if let Some(index) = indexes.get(name) {
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
                
                let index = Box::new(HnswIndex::new(
                    config.index_config.clone(),
                    config.distance_metric,
                    config.dimension,
                ));
                
                // TODO: Iterate through all vectors in storage and rebuild index
                // This would require implementing an iterator over stored vectors
                // For now, we create an empty index
                
                self.indexes.write().insert(collection_name.clone(), index);
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