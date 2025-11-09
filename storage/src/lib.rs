pub mod wal;
pub mod mmap;
pub mod recovery;
pub mod snapshot;

use vectordb_common::{Result, VectorDbError};
use vectordb_common::types::*;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

pub use wal::*;
pub use mmap::*;
pub use recovery::*;
pub use snapshot::*;

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
            data_dir: data_dir.clone(),
            collections: RwLock::new(HashMap::new()),
            wal,
        };

        // Step 1: Discover and load existing collections from metadata files
        engine.discover_collections().await?;

        // Step 2: Recover from WAL on startup (for any pending operations)
        engine.recover().await?;

        tracing::info!("StorageEngine initialized with {} collections", engine.collections.read().len());

        Ok(engine)
    }

    /// Discover existing collections by scanning the data directory for metadata files
    async fn discover_collections(&mut self) -> Result<()> {
        tracing::info!("Discovering existing collections in: {}", self.data_dir.display());

        let entries = match std::fs::read_dir(&self.data_dir) {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                tracing::info!("Data directory does not exist yet, skipping discovery");
                return Ok(());
            }
            Err(e) => return Err(VectorDbError::Io(e)),
        };

        let mut discovered_count = 0;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            // Skip non-directories and special directories
            if !path.is_dir() {
                continue;
            }

            let dir_name = match path.file_name().and_then(|n| n.to_str()) {
                Some(name) => name,
                None => continue,
            };

            // Skip special directories (WAL, backups, deleted)
            if dir_name == "wal" || dir_name.starts_with('.') {
                continue;
            }

            // Check for metadata.json file
            let metadata_path = path.join("metadata.json");
            if !metadata_path.exists() {
                tracing::warn!(
                    "Collection directory '{}' exists but has no metadata.json, skipping",
                    dir_name
                );
                continue;
            }

            // Load the collection
            match CollectionStorage::load(&path).await {
                Ok(storage) => {
                    let collection_name = storage.config().name.clone();
                    self.collections.write().insert(collection_name.clone(), Arc::new(storage));
                    discovered_count += 1;
                    tracing::info!("Discovered collection: {}", collection_name);
                }
                Err(e) => {
                    tracing::error!("Failed to load collection from '{}': {}", dir_name, e);
                    // Continue with other collections instead of failing completely
                }
            }
        }

        tracing::info!("Discovered {} collections from metadata files", discovered_count);
        Ok(())
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

    /// Get all vectors from a collection (used for index rebuilding)
    pub async fn get_all_vectors(&self, collection: &str) -> Result<Vec<Vector>> {
        let storage = {
            let collections = self.collections.read();
            collections
                .get(collection)
                .ok_or_else(|| VectorDbError::CollectionNotFound {
                    name: collection.to_string(),
                })?
                .clone()
        };

        storage.iter_vectors().await
    }

    /// Get recovery manager for backup/restore operations
    pub fn get_recovery_manager(&self) -> RecoveryManager {
        RecoveryManager::new(&self.data_dir)
    }

    /// Register an imported collection with the storage engine
    pub async fn register_imported_collection(&self, config: &CollectionConfig) -> Result<()> {
        // Check if collection already exists
        {
            let collections = self.collections.read();
            if collections.contains_key(&config.name) {
                return Err(VectorDbError::CollectionAlreadyExists {
                    name: config.name.clone(),
                });
            }
        }

        // Create storage for the imported collection
        let collection_dir = self.data_dir.join(&config.name);
        let storage = Arc::new(CollectionStorage::new(collection_dir, config.clone()).await?);

        // Log the operation to WAL
        let op = WALOperation::CreateCollection(config.clone());
        self.wal.append(&op).await?;

        // Register with collections
        self.collections.write().insert(config.name.clone(), storage);

        tracing::info!("Registered imported collection: {}", config.name);
        Ok(())
    }

    /// Get the data directory path
    pub fn get_data_dir(&self) -> &Path {
        &self.data_dir
    }

    /// Get collection directory path
    pub fn get_collection_dir(&self, collection: &str) -> Result<PathBuf> {
        Ok(self.data_dir.join(collection))
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
    metadata_path: PathBuf,
}

impl CollectionStorage {
    async fn new<P: AsRef<Path>>(dir: P, config: CollectionConfig) -> Result<Self> {
        let dir = dir.as_ref();
        std::fs::create_dir_all(dir)?;

        let data_path = dir.join("vectors.bin");
        let index_path = dir.join("index.bin");
        let metadata_path = dir.join("metadata.json");

        let data_file = MMapStorage::new(data_path).await?;
        let index_file = MMapStorage::new(index_path).await?;

        let storage = Self {
            config: config.clone(),
            data_file,
            index_file,
            metadata_path: metadata_path.clone(),
        };

        // Persist metadata to disk
        storage.save_metadata().await?;

        Ok(storage)
    }

    /// Load collection from existing directory (used during startup recovery)
    async fn load<P: AsRef<Path>>(dir: P) -> Result<Self> {
        let dir = dir.as_ref();
        let metadata_path = dir.join("metadata.json");

        // Load metadata from disk
        let metadata_content = std::fs::read_to_string(&metadata_path)
            .map_err(|e| VectorDbError::Io(e))?;

        let config: CollectionConfig = serde_json::from_str(&metadata_content)
            .map_err(|e| VectorDbError::Serialization(format!("Failed to deserialize metadata: {}", e)))?;

        let data_path = dir.join("vectors.bin");
        let index_path = dir.join("index.bin");

        let data_file = MMapStorage::new(data_path).await?;
        let index_file = MMapStorage::new(index_path).await?;

        tracing::info!("Loaded collection '{}' from metadata", config.name);

        Ok(Self {
            config,
            data_file,
            index_file,
            metadata_path,
        })
    }

    /// Save collection metadata to disk
    async fn save_metadata(&self) -> Result<()> {
        let metadata_json = serde_json::to_string_pretty(&self.config)
            .map_err(|e| VectorDbError::Serialization(format!("Failed to serialize metadata: {}", e)))?;

        std::fs::write(&self.metadata_path, metadata_json)
            .map_err(|e| VectorDbError::Io(e))?;

        tracing::debug!("Saved metadata for collection: {}", self.config.name);
        Ok(())
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

        // Write length prefix (4 bytes, u32 little-endian) + data
        let length = serialized.len() as u32;
        let mut record = Vec::with_capacity(4 + serialized.len());
        record.extend_from_slice(&length.to_le_bytes());
        record.extend_from_slice(&serialized);

        self.data_file.append(&record).await?;

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
        // Format: [length_prefix(4 bytes)][serialized_data][length_prefix][data]...
        let mut batch_buffer = Vec::with_capacity(vectors.len() * (self.config.dimension * 4 + 100));

        for vector in vectors {
            let serialized = bincode::serialize(vector)
                .map_err(|e| VectorDbError::Serialization(e.to_string()))?;

            // Write length prefix (4 bytes, u32 little-endian)
            let length = serialized.len() as u32;
            batch_buffer.extend_from_slice(&length.to_le_bytes());

            // Write serialized vector data
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

    /// Iterate over all vectors in the collection
    pub async fn iter_vectors(&self) -> Result<Vec<Vector>> {
        let mut vectors = Vec::new();
        let mut iter = self.data_file.iter().await?;

        while let Some(data) = iter.next().await? {
            match bincode::deserialize::<Vector>(&data) {
                Ok(vector) => vectors.push(vector),
                Err(e) => {
                    tracing::warn!(
                        "Failed to deserialize vector in collection '{}': {}",
                        self.config.name,
                        e
                    );
                    // Continue with next vector instead of failing completely
                }
            }
        }

        tracing::info!(
            "Loaded {} vectors from storage for collection '{}'",
            vectors.len(),
            self.config.name
        );

        Ok(vectors)
    }
}