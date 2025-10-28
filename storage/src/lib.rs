pub mod wal;
pub mod mmap;
pub mod recovery;

use vectordb_common::{Result, VectorDbError};
use vectordb_common::types::*;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

pub use wal::*;
pub use mmap::*;
pub use recovery::*;

/// Storage engine for persistent vector storage with WAL
pub struct StorageEngine {
    data_dir: PathBuf,
    collections: RwLock<HashMap<CollectionId, Arc<CollectionStorage>>>,
    wal: WriteAheadLog,
}

impl StorageEngine {
    pub async fn new<P: AsRef<Path>>(data_dir: P) -> Result<Self> {
        let data_dir = data_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&data_dir)?;
        
        let wal_path = data_dir.join("wal");
        let wal = WriteAheadLog::new(wal_path).await?;
        
        let mut engine = Self {
            data_dir,
            collections: RwLock::new(HashMap::new()),
            wal,
        };
        
        // Recover from WAL on startup
        engine.recover().await?;
        
        Ok(engine)
    }
    
    pub async fn create_collection(&self, config: &CollectionConfig) -> Result<()> {
        // Check if collection already exists without holding write lock
        {
            let collections = self.collections.read();
            if collections.contains_key(&config.name) {
                return Err(VectorDbError::CollectionAlreadyExists {
                    name: config.name.clone(),
                });
            }
        }
        
        // Create storage without holding any locks
        let collection_dir = self.data_dir.join(&config.name);
        let storage = Arc::new(CollectionStorage::new(collection_dir, config.clone()).await?);
        
        // Log the operation (await without holding any locks)
        let op = WALOperation::CreateCollection(config.clone());
        self.wal.append(&op).await?;
        
        // Now insert with write lock
        self.collections.write().insert(config.name.clone(), storage);
        
        tracing::info!("Created collection: {}", config.name);
        Ok(())
    }
    
    pub async fn delete_collection(&self, name: &str) -> Result<()> {
        // Check if collection exists first without holding the lock
        {
            let collections = self.collections.read();
            if !collections.contains_key(name) {
                return Err(VectorDbError::CollectionNotFound {
                    name: name.to_string(),
                });
            }
        }
        
        // Log the operation (await without holding any locks)
        let op = WALOperation::DeleteCollection(name.to_string());
        self.wal.append(&op).await?;
        
        // Now remove from collections with write lock
        self.collections.write().remove(name);
        
        // Remove collection directory
        let collection_dir = self.data_dir.join(name);
        if collection_dir.exists() {
            std::fs::remove_dir_all(collection_dir)?;
        }
        
        tracing::info!("Deleted collection: {}", name);
        Ok(())
    }
    
    pub async fn insert_vector(&self, collection: &str, vector: &Vector) -> Result<()> {
        // Clone the storage reference to avoid holding the lock across await points
        let storage = {
            let collections = self.collections.read();
            collections
                .get(collection)
                .ok_or_else(|| VectorDbError::CollectionNotFound {
                    name: collection.to_string(),
                })?
                .clone()
        };
        
        // Log the operation
        let op = WALOperation::InsertVector {
            collection: collection.to_string(),
            vector: vector.clone(),
        };
        self.wal.append(&op).await?;
        
        storage.insert(vector).await?;
        
        Ok(())
    }
    
    pub async fn batch_insert(&self, collection: &str, vectors: &[Vector]) -> Result<()> {
        // Clone the storage reference to avoid holding the lock across await points
        let storage = {
            let collections = self.collections.read();
            collections
                .get(collection)
                .ok_or_else(|| VectorDbError::CollectionNotFound {
                    name: collection.to_string(),
                })?
                .clone()
        };

        // PERFORMANCE OPTIMIZATION: Write to storage first, WAL second
        // This reduces latency by not blocking on WAL sync for every batch
        // The WAL is buffered and will be flushed periodically
        storage.batch_insert(vectors).await?;

        // Log the operation (async, buffered - doesn't block)
        let op = WALOperation::BatchInsert {
            collection: collection.to_string(),
            vectors: vectors.to_vec(),
        };
        self.wal.append(&op).await?;

        Ok(())
    }
    
    pub async fn get_vector(&self, collection: &str, id: &VectorId) -> Result<Option<Vector>> {
        // Clone the storage reference to avoid holding the lock across await points
        let storage = {
            let collections = self.collections.read();
            collections
                .get(collection)
                .ok_or_else(|| VectorDbError::CollectionNotFound {
                    name: collection.to_string(),
                })?
                .clone()
        };
        
        storage.get(id).await
    }
    
    pub async fn delete_vector(&self, collection: &str, id: &VectorId) -> Result<bool> {
        // Clone the storage reference to avoid holding the lock across await points
        let storage = {
            let collections = self.collections.read();
            collections
                .get(collection)
                .ok_or_else(|| VectorDbError::CollectionNotFound {
                    name: collection.to_string(),
                })?
                .clone()
        };
        
        // Log the operation
        let op = WALOperation::DeleteVector {
            collection: collection.to_string(),
            id: *id,
        };
        self.wal.append(&op).await?;
        
        storage.delete(id).await
    }
    
    pub fn list_collections(&self) -> Vec<CollectionId> {
        let collections = self.collections.read();
        collections.keys().cloned().collect()
    }
    
    pub fn get_collection_config(&self, name: &str) -> Result<Option<CollectionConfig>> {
        let collections = self.collections.read();
        Ok(collections.get(name).map(|s| s.config().clone()))
    }
    
    pub async fn get_collection_stats(&self, name: &str) -> Result<Option<CollectionStats>> {
        // We need to restructure this to avoid holding the lock across the await
        // For now, let's get the config synchronously and compute stats differently
        let config = {
            let collections = self.collections.read();
            collections.get(name).map(|s| s.config().clone())
        };
        
        if let Some(_config) = config {
            // TODO: Implement proper async stats collection without holding locks
            // For now return a basic stats structure
            Ok(Some(CollectionStats {
                name: name.to_string(),
                vector_count: 0,  // TODO: get actual count
                dimension: _config.dimension,
                index_size: 0,   // TODO: get actual size
                memory_usage: 0, // TODO: get actual usage
            }))
        } else {
            Ok(None)
        }
    }
    
    pub async fn sync(&self) -> Result<()> {
        self.wal.sync().await?;
        
        // Clone all storage references to avoid holding the lock across await points
        let storages: Vec<Arc<CollectionStorage>> = {
            let collections = self.collections.read();
            collections.values().cloned().collect()
        };
        
        for storage in storages {
            storage.sync().await?;
        }
        
        Ok(())
    }
    
    async fn recover(&mut self) -> Result<()> {
        let recovery = RecoveryManager::new(&self.data_dir);
        let operations = recovery.recover_from_wal(&self.wal).await?;
        
        tracing::info!("Recovering {} operations from WAL", operations.len());
        
        for op in operations {
            self.apply_operation(op).await?;
        }
        
        Ok(())
    }
    
    async fn apply_operation(&mut self, op: WALOperation) -> Result<()> {
        match op {
            WALOperation::CreateCollection(config) => {
                let collection_dir = self.data_dir.join(&config.name);
                let storage = Arc::new(CollectionStorage::new(collection_dir, config.clone()).await?);
                self.collections.write().insert(config.name.clone(), storage);
            }
            WALOperation::DeleteCollection(name) => {
                self.collections.write().remove(&name);
            }
            WALOperation::InsertVector { collection, vector } => {
                // Clone the storage reference to avoid holding the lock across await points
                let storage = {
                    let collections = self.collections.read();
                    collections.get(&collection).cloned()
                };
                
                if let Some(storage) = storage {
                    storage.insert(&vector).await?;
                }
            }
            WALOperation::BatchInsert { collection, vectors } => {
                // Clone the storage reference to avoid holding the lock across await points
                let storage = {
                    let collections = self.collections.read();
                    collections.get(&collection).cloned()
                };
                
                if let Some(storage) = storage {
                    storage.batch_insert(&vectors).await?;
                }
            }
            WALOperation::DeleteVector { collection, id } => {
                // Clone the storage reference to avoid holding the lock across await points
                let storage = {
                    let collections = self.collections.read();
                    collections.get(&collection).cloned()
                };
                
                if let Some(storage) = storage {
                    storage.delete(&id).await?;
                }
            }
        }
        
        Ok(())
    }
}

/// Storage for a single collection
pub struct CollectionStorage {
    config: CollectionConfig,
    data_file: MMapStorage,
    index_file: MMapStorage,
}

impl CollectionStorage {
    async fn new<P: AsRef<Path>>(dir: P, config: CollectionConfig) -> Result<Self> {
        let dir = dir.as_ref();
        std::fs::create_dir_all(dir)?;
        
        let data_path = dir.join("vectors.bin");
        let index_path = dir.join("index.bin");
        
        let data_file = MMapStorage::new(data_path).await?;
        let index_file = MMapStorage::new(index_path).await?;
        
        Ok(Self {
            config,
            data_file,
            index_file,
        })
    }
    
    fn config(&self) -> &CollectionConfig {
        &self.config
    }
    
    async fn insert(&self, vector: &Vector) -> Result<()> {
        if vector.data.len() != self.config.dimension {
            return Err(VectorDbError::InvalidDimension {
                expected: self.config.dimension,
                actual: vector.data.len(),
            });
        }
        
        let serialized = bincode::serialize(vector)
            .map_err(|e| VectorDbError::Serialization(e.to_string()))?;
        
        self.data_file.append(&serialized).await?;
        
        Ok(())
    }
    
    async fn batch_insert(&self, vectors: &[Vector]) -> Result<()> {
        if vectors.is_empty() {
            return Ok(());
        }

        // Validate all vectors first
        for vector in vectors {
            if vector.data.len() != self.config.dimension {
                return Err(VectorDbError::InvalidDimension {
                    expected: self.config.dimension,
                    actual: vector.data.len(),
                });
            }
        }

        // Serialize all vectors into a single buffer to reduce async calls
        // This is much faster than calling append() for each vector
        let mut batch_buffer = Vec::with_capacity(vectors.len() * (self.config.dimension * 4 + 100));

        for vector in vectors {
            let serialized = bincode::serialize(vector)
                .map_err(|e| VectorDbError::Serialization(e.to_string()))?;
            batch_buffer.extend_from_slice(&serialized);
        }

        // Single async write for entire batch
        self.data_file.append(&batch_buffer).await?;

        Ok(())
    }
    
    async fn get(&self, _id: &VectorId) -> Result<Option<Vector>> {
        // TODO: Implement efficient lookup using index
        // For now, this is a placeholder
        Ok(None)
    }
    
    async fn delete(&self, _id: &VectorId) -> Result<bool> {
        // TODO: Implement deletion with tombstone markers
        // For now, this is a placeholder
        Ok(false)
    }
    
    async fn stats(&self) -> Result<CollectionStats> {
        Ok(CollectionStats {
            name: self.config.name.clone(),
            vector_count: 0, // TODO: Track count
            dimension: self.config.dimension,
            index_size: self.index_file.size().await? as usize,
            memory_usage: (self.data_file.size().await? + self.index_file.size().await?) as usize,
        })
    }
    
    async fn sync(&self) -> Result<()> {
        self.data_file.sync().await?;
        self.index_file.sync().await?;
        Ok(())
    }
}