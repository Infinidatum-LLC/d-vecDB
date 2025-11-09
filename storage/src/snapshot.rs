use std::path::{Path, PathBuf};
use std::fs;
use std::io::{self, Write};
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use vectordb_common::{Result, VectorDbError};

/// Snapshot metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMetadata {
    pub name: String,
    pub collection: String,
    pub created_at: u64,
    pub size_bytes: u64,
    pub vector_count: usize,
    pub checksum: String,
}

/// Snapshot manager for creating and restoring point-in-time snapshots
pub struct SnapshotManager {
    snapshots_dir: PathBuf,
}

impl SnapshotManager {
    /// Create a new snapshot manager
    pub fn new<P: AsRef<Path>>(data_dir: P) -> io::Result<Self> {
        let snapshots_dir = data_dir.as_ref().join("snapshots");
        fs::create_dir_all(&snapshots_dir)?;

        Ok(Self { snapshots_dir })
    }

    /// Create a snapshot of a collection
    pub async fn create_snapshot(
        &self,
        collection_name: &str,
        collection_dir: &Path,
    ) -> Result<SnapshotMetadata> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let snapshot_name = format!("{}_{}", collection_name, timestamp);
        let snapshot_dir = self.snapshots_dir.join(&snapshot_name);

        // Create snapshot directory
        fs::create_dir_all(&snapshot_dir).map_err(|e| VectorDbError::Io {
            message: format!("Failed to create snapshot directory: {}", e),
        })?;

        // Copy collection files
        let mut total_size = 0u64;
        let mut vector_count = 0usize;

        // Copy vectors.bin
        if let Ok(src) = collection_dir.join("vectors.bin").canonicalize() {
            let dst = snapshot_dir.join("vectors.bin");
            fs::copy(&src, &dst).map_err(|e| VectorDbError::Io {
                message: format!("Failed to copy vectors.bin: {}", e),
            })?;
            total_size += fs::metadata(&dst).map(|m| m.len()).unwrap_or(0);
        }

        // Copy WAL
        if let Ok(src) = collection_dir.join("wal.log").canonicalize() {
            let dst = snapshot_dir.join("wal.log");
            fs::copy(&src, &dst).map_err(|e| VectorDbError::Io {
                message: format!("Failed to copy wal.log: {}", e),
            })?;
            total_size += fs::metadata(&dst).map(|m| m.len()).unwrap_or(0);
        }

        // Copy metadata
        if let Ok(src) = collection_dir.join("metadata.json").canonicalize() {
            let dst = snapshot_dir.join("metadata.json");
            fs::copy(&src, &dst).map_err(|e| VectorDbError::Io {
                message: format!("Failed to copy metadata.json: {}", e),
            })?;
            total_size += fs::metadata(&dst).map(|m| m.len()).unwrap_or(0);
        }

        // Calculate checksum
        let checksum = self.calculate_checksum(&snapshot_dir)?;

        // Create snapshot metadata
        let metadata = SnapshotMetadata {
            name: snapshot_name.clone(),
            collection: collection_name.to_string(),
            created_at: timestamp,
            size_bytes: total_size,
            vector_count,
            checksum,
        };

        // Save metadata
        let metadata_path = snapshot_dir.join("snapshot.json");
        let metadata_json = serde_json::to_string_pretty(&metadata).map_err(|e| {
            VectorDbError::Serialization {
                message: format!("Failed to serialize snapshot metadata: {}", e),
            }
        })?;

        fs::write(&metadata_path, metadata_json).map_err(|e| VectorDbError::Io {
            message: format!("Failed to write snapshot metadata: {}", e),
        })?;

        tracing::info!("Created snapshot '{}' for collection '{}'", snapshot_name, collection_name);

        Ok(metadata)
    }

    /// List all snapshots
    pub fn list_snapshots(&self) -> Result<Vec<SnapshotMetadata>> {
        let mut snapshots = Vec::new();

        let entries = fs::read_dir(&self.snapshots_dir).map_err(|e| VectorDbError::Io {
            message: format!("Failed to read snapshots directory: {}", e),
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| VectorDbError::Io {
                message: format!("Failed to read snapshot entry: {}", e),
            })?;

            let path = entry.path();
            if path.is_dir() {
                let metadata_path = path.join("snapshot.json");
                if metadata_path.exists() {
                    let metadata_json = fs::read_to_string(&metadata_path).map_err(|e| {
                        VectorDbError::Io {
                            message: format!("Failed to read snapshot metadata: {}", e),
                        }
                    })?;

                    let metadata: SnapshotMetadata =
                        serde_json::from_str(&metadata_json).map_err(|e| {
                            VectorDbError::Serialization {
                                message: format!("Failed to parse snapshot metadata: {}", e),
                            }
                        })?;

                    snapshots.push(metadata);
                }
            }
        }

        // Sort by creation time (newest first)
        snapshots.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        Ok(snapshots)
    }

    /// Get snapshot by name
    pub fn get_snapshot(&self, snapshot_name: &str) -> Result<SnapshotMetadata> {
        let metadata_path = self.snapshots_dir.join(snapshot_name).join("snapshot.json");

        if !metadata_path.exists() {
            return Err(VectorDbError::NotFound {
                message: format!("Snapshot '{}' not found", snapshot_name),
            });
        }

        let metadata_json = fs::read_to_string(&metadata_path).map_err(|e| VectorDbError::Io {
            message: format!("Failed to read snapshot metadata: {}", e),
        })?;

        let metadata: SnapshotMetadata =
            serde_json::from_str(&metadata_json).map_err(|e| VectorDbError::Serialization {
                message: format!("Failed to parse snapshot metadata: {}", e),
            })?;

        Ok(metadata)
    }

    /// Restore collection from snapshot
    pub async fn restore_snapshot(
        &self,
        snapshot_name: &str,
        target_dir: &Path,
    ) -> Result<()> {
        let snapshot_dir = self.snapshots_dir.join(snapshot_name);

        if !snapshot_dir.exists() {
            return Err(VectorDbError::NotFound {
                message: format!("Snapshot '{}' not found", snapshot_name),
            });
        }

        // Verify checksum
        let metadata = self.get_snapshot(snapshot_name)?;
        let current_checksum = self.calculate_checksum(&snapshot_dir)?;

        if current_checksum != metadata.checksum {
            return Err(VectorDbError::Corruption {
                message: format!("Snapshot checksum mismatch: expected {}, got {}", metadata.checksum, current_checksum),
            });
        }

        // Create target directory
        fs::create_dir_all(target_dir).map_err(|e| VectorDbError::Io {
            message: format!("Failed to create target directory: {}", e),
        })?;

        // Copy snapshot files to target
        for entry in fs::read_dir(&snapshot_dir).map_err(|e| VectorDbError::Io {
            message: format!("Failed to read snapshot directory: {}", e),
        })? {
            let entry = entry.map_err(|e| VectorDbError::Io {
                message: format!("Failed to read snapshot entry: {}", e),
            })?;

            let path = entry.path();
            let filename = path.file_name().unwrap();

            // Skip snapshot.json metadata file
            if filename == "snapshot.json" {
                continue;
            }

            let target_path = target_dir.join(filename);
            fs::copy(&path, &target_path).map_err(|e| VectorDbError::Io {
                message: format!("Failed to copy snapshot file: {}", e),
            })?;
        }

        tracing::info!("Restored snapshot '{}' to {}", snapshot_name, target_dir.display());

        Ok(())
    }

    /// Delete a snapshot
    pub fn delete_snapshot(&self, snapshot_name: &str) -> Result<()> {
        let snapshot_dir = self.snapshots_dir.join(snapshot_name);

        if !snapshot_dir.exists() {
            return Err(VectorDbError::NotFound {
                message: format!("Snapshot '{}' not found", snapshot_name),
            });
        }

        fs::remove_dir_all(&snapshot_dir).map_err(|e| VectorDbError::Io {
            message: format!("Failed to delete snapshot: {}", e),
        })?;

        tracing::info!("Deleted snapshot '{}'", snapshot_name);

        Ok(())
    }

    /// Calculate checksum for snapshot directory
    fn calculate_checksum(&self, snapshot_dir: &Path) -> Result<String> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();

        // Hash all file contents
        for entry in fs::read_dir(snapshot_dir).map_err(|e| VectorDbError::Io {
            message: format!("Failed to read snapshot directory: {}", e),
        })? {
            let entry = entry.map_err(|e| VectorDbError::Io {
                message: format!("Failed to read entry: {}", e),
            })?;

            let path = entry.path();
            if path.is_file() && path.file_name().unwrap() != "snapshot.json" {
                let contents = fs::read(&path).map_err(|e| VectorDbError::Io {
                    message: format!("Failed to read file for checksum: {}", e),
                })?;
                contents.hash(&mut hasher);
            }
        }

        Ok(format!("{:x}", hasher.finish()))
    }

    /// Clean up old snapshots (keep only N most recent)
    pub fn cleanup_old_snapshots(&self, keep_count: usize) -> Result<usize> {
        let mut snapshots = self.list_snapshots()?;

        if snapshots.len() <= keep_count {
            return Ok(0);
        }

        let to_delete = snapshots.split_off(keep_count);
        let deleted_count = to_delete.len();

        for snapshot in to_delete {
            self.delete_snapshot(&snapshot.name)?;
        }

        tracing::info!("Cleaned up {} old snapshots", deleted_count);

        Ok(deleted_count)
    }

    /// Export snapshot to tar.gz archive
    pub fn export_snapshot(&self, snapshot_name: &str, output_path: &Path) -> Result<()> {
        let snapshot_dir = self.snapshots_dir.join(snapshot_name);

        if !snapshot_dir.exists() {
            return Err(VectorDbError::NotFound {
                message: format!("Snapshot '{}' not found", snapshot_name),
            });
        }

        // Create tar.gz archive
        let tar_file = fs::File::create(output_path).map_err(|e| VectorDbError::Io {
            message: format!("Failed to create archive file: {}", e),
        })?;

        let enc = flate2::write::GzEncoder::new(tar_file, flate2::Compression::default());
        let mut tar = tar::Builder::new(enc);

        tar.append_dir_all(snapshot_name, &snapshot_dir)
            .map_err(|e| VectorDbError::Io {
                message: format!("Failed to create tar archive: {}", e),
            })?;

        tar.finish().map_err(|e| VectorDbError::Io {
            message: format!("Failed to finish tar archive: {}", e),
        })?;

        tracing::info!("Exported snapshot '{}' to {}", snapshot_name, output_path.display());

        Ok(())
    }

    /// Import snapshot from tar.gz archive
    pub fn import_snapshot(&self, archive_path: &Path) -> Result<String> {
        let tar_file = fs::File::open(archive_path).map_err(|e| VectorDbError::Io {
            message: format!("Failed to open archive file: {}", e),
        })?;

        let dec = flate2::read::GzDecoder::new(tar_file);
        let mut tar = tar::Archive::new(dec);

        tar.unpack(&self.snapshots_dir)
            .map_err(|e| VectorDbError::Io {
                message: format!("Failed to extract tar archive: {}", e),
            })?;

        // Find the imported snapshot name
        let archive_name = archive_path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| VectorDbError::Configuration {
                message: "Invalid archive filename".to_string(),
            })?;

        tracing::info!("Imported snapshot from {}", archive_path.display());

        Ok(archive_name.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_snapshot_creation() {
        let temp_dir = tempdir().unwrap();
        let manager = SnapshotManager::new(temp_dir.path()).unwrap();

        let collection_dir = temp_dir.path().join("test_collection");
        fs::create_dir_all(&collection_dir).unwrap();

        // Create some test files
        fs::write(collection_dir.join("vectors.bin"), b"test vectors").unwrap();
        fs::write(collection_dir.join("metadata.json"), b"{}").unwrap();

        let metadata = manager
            .create_snapshot("test_collection", &collection_dir)
            .await
            .unwrap();

        assert_eq!(metadata.collection, "test_collection");
        assert!(metadata.size_bytes > 0);
    }

    #[tokio::test]
    async fn test_snapshot_list() {
        let temp_dir = tempdir().unwrap();
        let manager = SnapshotManager::new(temp_dir.path()).unwrap();

        let collection_dir = temp_dir.path().join("test_collection");
        fs::create_dir_all(&collection_dir).unwrap();
        fs::write(collection_dir.join("vectors.bin"), b"test").unwrap();

        manager
            .create_snapshot("test_collection", &collection_dir)
            .await
            .unwrap();

        let snapshots = manager.list_snapshots().unwrap();
        assert_eq!(snapshots.len(), 1);
    }
}
