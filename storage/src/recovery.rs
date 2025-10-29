use vectordb_common::{Result, VectorDbError};
use crate::wal::{WriteAheadLog, WALOperation};
use std::path::{Path, PathBuf};
use std::collections::HashSet;
use tokio::fs;
use tracing::{info, warn};

/// Recovery manager for crash recovery and consistency
pub struct RecoveryManager {
    data_dir: PathBuf,
}

impl RecoveryManager {
    pub fn new<P: AsRef<Path>>(data_dir: P) -> Self {
        Self {
            data_dir: data_dir.as_ref().to_path_buf(),
        }
    }
    
    /// Recover from WAL after a crash
    pub async fn recover_from_wal(&self, wal: &WriteAheadLog) -> Result<Vec<WALOperation>> {
        info!("Starting crash recovery from WAL");
        
        // Read all operations from WAL
        let operations = wal.read_all().await?;
        
        if operations.is_empty() {
            info!("No operations found in WAL, recovery complete");
            return Ok(operations);
        }
        
        // Validate operations and check for consistency
        let validated_ops = self.validate_operations(&operations).await?;
        
        info!("Recovered {} valid operations from WAL", validated_ops.len());
        Ok(validated_ops)
    }
    
    /// Validate operations for consistency
    async fn validate_operations(&self, operations: &[WALOperation]) -> Result<Vec<WALOperation>> {
        let mut valid_ops = Vec::new();
        let mut existing_collections = self.discover_existing_collections().await?;
        
        for (i, op) in operations.iter().enumerate() {
            match self.validate_operation(op, &mut existing_collections).await {
                Ok(()) => {
                    valid_ops.push(op.clone());
                }
                Err(e) => {
                    warn!("Invalid operation at position {}: {:?} - {}", i, op, e);
                    // Continue with remaining operations
                }
            }
        }
        
        Ok(valid_ops)
    }
    
    /// Validate a single operation
    async fn validate_operation(
        &self,
        op: &WALOperation,
        existing_collections: &mut HashSet<String>,
    ) -> Result<()> {
        match op {
            WALOperation::CreateCollection(config) => {
                if existing_collections.contains(&config.name) {
                    return Err(VectorDbError::Internal {
                        message: format!("Collection {} already exists", config.name),
                    });
                }
                existing_collections.insert(config.name.clone());
            }
            WALOperation::DeleteCollection(name) => {
                if !existing_collections.contains(name) {
                    return Err(VectorDbError::Internal {
                        message: format!("Collection {} does not exist", name),
                    });
                }
                existing_collections.remove(name);
            }
            WALOperation::InsertVector { collection, vector: _ } => {
                if !existing_collections.contains(collection) {
                    return Err(VectorDbError::Internal {
                        message: format!("Collection {} does not exist", collection),
                    });
                }
                // Additional vector validation could go here
            }
            WALOperation::BatchInsert { collection, vectors } => {
                if !existing_collections.contains(collection) {
                    return Err(VectorDbError::Internal {
                        message: format!("Collection {} does not exist", collection),
                    });
                }
                // Validate all vectors in the batch
                for vector in vectors {
                    if vector.data.is_empty() {
                        return Err(VectorDbError::Internal {
                            message: "Empty vector in batch".to_string(),
                        });
                    }
                }
            }
            WALOperation::DeleteVector { collection, .. } => {
                if !existing_collections.contains(collection) {
                    return Err(VectorDbError::Internal {
                        message: format!("Collection {} does not exist", collection),
                    });
                }
            }
        }
        
        Ok(())
    }
    
    /// Discover collections that exist on disk
    async fn discover_existing_collections(&self) -> Result<HashSet<String>> {
        let mut collections = HashSet::new();
        
        if !self.data_dir.exists() {
            return Ok(collections);
        }
        
        let mut entries = fs::read_dir(&self.data_dir).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            
            if path.is_dir() {
                // Check if it looks like a collection directory
                let vectors_file = path.join("vectors.bin");
                let index_file = path.join("index.bin");
                
                if vectors_file.exists() || index_file.exists() {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        collections.insert(name.to_string());
                    }
                }
            }
        }
        
        info!("Discovered {} existing collections on disk", collections.len());
        Ok(collections)
    }
    
    /// Perform consistency check on collections
    pub async fn check_consistency(&self) -> Result<Vec<String>> {
        let mut issues = Vec::new();
        let collections = self.discover_existing_collections().await?;
        
        for collection in &collections {
            if let Err(e) = self.check_collection_consistency(collection).await {
                let issue = format!("Collection {}: {}", collection, e);
                issues.push(issue);
            }
        }
        
        if issues.is_empty() {
            info!("All collections passed consistency check");
        } else {
            warn!("Found {} consistency issues", issues.len());
            for issue in &issues {
                warn!("{}", issue);
            }
        }
        
        Ok(issues)
    }
    
    /// Check consistency of a single collection
    async fn check_collection_consistency(&self, collection: &str) -> Result<()> {
        let collection_dir = self.data_dir.join(collection);
        
        // Check if required files exist
        let vectors_file = collection_dir.join("vectors.bin");
        let index_file = collection_dir.join("index.bin");
        
        if !vectors_file.exists() && !index_file.exists() {
            return Err(VectorDbError::StorageError {
                message: "No data files found".to_string(),
            });
        }
        
        // Check file sizes and basic integrity
        if vectors_file.exists() {
            let metadata = fs::metadata(&vectors_file).await?;
            if metadata.len() == 0 {
                return Err(VectorDbError::StorageError {
                    message: "Empty vectors file".to_string(),
                });
            }
        }
        
        // Additional checks could include:
        // - Vector count consistency between data and index
        // - Index structure validation
        // - Data format validation
        
        Ok(())
    }
    
    /// Create backup of data directory
    pub async fn create_backup<P: AsRef<Path>>(&self, backup_path: P) -> Result<()> {
        let backup_path = backup_path.as_ref();

        info!("Creating backup at: {}", backup_path.display());

        // Create backup directory
        fs::create_dir_all(backup_path).await?;

        // Copy all collection directories
        let collections = self.discover_existing_collections().await?;

        for collection in &collections {
            let src_dir = self.data_dir.join(collection);
            let dst_dir = backup_path.join(collection);

            self.copy_dir_recursive(&src_dir, &dst_dir).await?;
        }

        info!("Backup completed successfully");
        Ok(())
    }

    /// Backup a single collection before destructive operation
    pub async fn backup_collection(&self, collection_name: &str) -> Result<PathBuf> {
        use chrono::Local;

        let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
        let backup_dir = self.data_dir.join(".backups").join(format!("{}_{}", collection_name, timestamp));

        info!("Creating pre-operation backup for collection '{}' at: {}", collection_name, backup_dir.display());

        let src_dir = self.data_dir.join(collection_name);
        if !src_dir.exists() {
            return Err(VectorDbError::CollectionNotFound {
                name: collection_name.to_string(),
            });
        }

        // Create backup directory
        fs::create_dir_all(&backup_dir).await?;

        // Copy collection directory
        self.copy_dir_recursive(&src_dir, &backup_dir).await?;

        info!("Pre-operation backup completed: {}", backup_dir.display());
        Ok(backup_dir)
    }

    /// Soft delete a collection (move to .deleted with timestamp)
    pub async fn soft_delete_collection(&self, collection_name: &str) -> Result<PathBuf> {
        use chrono::Local;

        let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
        let deleted_dir = self.data_dir.join(".deleted").join(format!("{}_{}", collection_name, timestamp));

        info!("Soft deleting collection '{}' to: {}", collection_name, deleted_dir.display());

        let src_dir = self.data_dir.join(collection_name);
        if !src_dir.exists() {
            return Err(VectorDbError::CollectionNotFound {
                name: collection_name.to_string(),
            });
        }

        // Create .deleted directory
        fs::create_dir_all(self.data_dir.join(".deleted")).await?;

        // Move collection directory to .deleted
        fs::rename(&src_dir, &deleted_dir).await?;

        info!("Collection soft deleted successfully. Recoverable for 24 hours at: {}", deleted_dir.display());
        Ok(deleted_dir)
    }

    /// Restore a soft-deleted collection
    pub async fn restore_collection(&self, deleted_path: &Path, collection_name: Option<&str>) -> Result<String> {
        if !deleted_path.exists() {
            return Err(VectorDbError::Internal {
                message: format!("Backup/deleted directory not found: {}", deleted_path.display()),
            });
        }

        // Extract collection name from path if not provided
        let name = if let Some(n) = collection_name {
            n.to_string()
        } else {
            // Extract from path like "incidents_20251029_074542"
            deleted_path
                .file_name()
                .and_then(|n| n.to_str())
                .and_then(|s| s.split('_').next())
                .ok_or_else(|| VectorDbError::Internal {
                    message: "Cannot determine collection name from path".to_string(),
                })?
                .to_string()
        };

        let restore_dir = self.data_dir.join(&name);

        // Check if collection already exists
        if restore_dir.exists() {
            return Err(VectorDbError::Internal {
                message: format!("Collection '{}' already exists. Delete it first or use a different name.", name),
            });
        }

        info!("Restoring collection '{}' from: {}", name, deleted_path.display());

        // Copy from backup/deleted to active collection
        self.copy_dir_recursive(deleted_path, &restore_dir).await?;

        info!("Collection '{}' restored successfully", name);
        Ok(name)
    }

    /// Import orphaned collection data (vectors.bin/index.bin files)
    pub async fn import_orphaned_collection(&self, orphaned_dir: &Path, new_collection_name: &str) -> Result<()> {
        info!("Importing orphaned collection from: {} as '{}'", orphaned_dir.display(), new_collection_name);

        // Validate orphaned directory has required files
        let vectors_file = orphaned_dir.join("vectors.bin");
        let index_file = orphaned_dir.join("index.bin");

        if !vectors_file.exists() && !index_file.exists() {
            return Err(VectorDbError::Internal {
                message: format!("No vectors.bin or index.bin found in {}", orphaned_dir.display()),
            });
        }

        let restore_dir = self.data_dir.join(new_collection_name);

        // Check if collection already exists
        if restore_dir.exists() {
            return Err(VectorDbError::Internal {
                message: format!("Collection '{}' already exists. Use a different name or delete existing collection.", new_collection_name),
            });
        }

        // Copy orphaned data to new collection directory
        fs::create_dir_all(&restore_dir).await?;

        if vectors_file.exists() {
            let dst_vectors = restore_dir.join("vectors.bin");
            fs::copy(&vectors_file, &dst_vectors).await?;
            let size_mb = fs::metadata(&vectors_file).await?.len() as f64 / 1_048_576.0;
            info!("Copied vectors.bin ({:.2} MB)", size_mb);
        }

        if index_file.exists() {
            let dst_index = restore_dir.join("index.bin");
            fs::copy(&index_file, &dst_index).await?;
            let size_mb = fs::metadata(&index_file).await?.len() as f64 / 1_048_576.0;
            info!("Copied index.bin ({:.2} MB)", size_mb);
        }

        info!("Orphaned collection imported successfully as '{}'", new_collection_name);
        Ok(())
    }

    /// List all soft-deleted collections
    pub async fn list_deleted_collections(&self) -> Result<Vec<(String, PathBuf, u64)>> {
        let deleted_dir = self.data_dir.join(".deleted");

        if !deleted_dir.exists() {
            return Ok(Vec::new());
        }

        let mut deleted = Vec::new();
        let mut entries = fs::read_dir(&deleted_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_dir() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    let metadata = fs::metadata(&path).await?;
                    let modified = metadata.modified()?
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs();

                    deleted.push((name.to_string(), path, modified));
                }
            }
        }

        Ok(deleted)
    }

    /// Cleanup old soft-deleted collections (older than 24 hours)
    pub async fn cleanup_old_deleted(&self, retention_hours: u64) -> Result<Vec<String>> {
        let deleted = self.list_deleted_collections().await?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let retention_seconds = retention_hours * 3600;
        let mut cleaned = Vec::new();

        for (name, path, modified) in deleted {
            let age_seconds = now.saturating_sub(modified);

            if age_seconds > retention_seconds {
                info!("Cleaning up old deleted collection: {} (age: {:.1} hours)", name, age_seconds as f64 / 3600.0);
                fs::remove_dir_all(&path).await?;
                cleaned.push(name);
            }
        }

        info!("Cleaned up {} old deleted collections", cleaned.len());
        Ok(cleaned)
    }
    
    /// Recursively copy directory
    async fn copy_dir_recursive(&self, src: &Path, dst: &Path) -> Result<()> {
        use std::collections::VecDeque;
        
        let mut queue = VecDeque::new();
        queue.push_back((src.to_path_buf(), dst.to_path_buf()));
        
        while let Some((src_path, dst_path)) = queue.pop_front() {
            fs::create_dir_all(&dst_path).await?;
            
            let mut entries = fs::read_dir(&src_path).await?;
            
            while let Some(entry) = entries.next_entry().await? {
                let entry_src = entry.path();
                let entry_dst = dst_path.join(entry.file_name());
                
                if entry_src.is_dir() {
                    queue.push_back((entry_src, entry_dst));
                } else {
                    fs::copy(&entry_src, &entry_dst).await?;
                }
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use vectordb_common::types::*;
    
    #[tokio::test]
    async fn test_discover_collections() {
        let temp_dir = tempdir().unwrap();
        let recovery = RecoveryManager::new(temp_dir.path());
        
        // Create a mock collection directory
        let collection_dir = temp_dir.path().join("test_collection");
        fs::create_dir_all(&collection_dir).await.unwrap();
        fs::File::create(collection_dir.join("vectors.bin")).await.unwrap();
        
        let collections = recovery.discover_existing_collections().await.unwrap();
        assert!(collections.contains("test_collection"));
    }
    
    #[tokio::test]
    async fn test_validate_operations() {
        let temp_dir = tempdir().unwrap();
        let recovery = RecoveryManager::new(temp_dir.path());
        
        let config = CollectionConfig {
            name: "test".to_string(),
            dimension: 128,
            distance_metric: DistanceMetric::Cosine,
            vector_type: VectorType::Float32,
            index_config: IndexConfig::default(),
        };
        
        let operations = vec![
            WALOperation::CreateCollection(config.clone()),
            WALOperation::InsertVector {
                collection: "test".to_string(),
                vector: Vector {
                    id: uuid::Uuid::new_v4(),
                    data: vec![0.1; 128],
                    metadata: None,
                },
            },
        ];
        
        let validated = recovery.validate_operations(&operations).await.unwrap();
        assert_eq!(validated.len(), 2);
    }
}