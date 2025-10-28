use vectordb_common::{Result, VectorDbError};
use vectordb_common::types::*;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::sync::Mutex;
use uuid::Uuid;
use std::sync::Arc;

/// Write-Ahead Log operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WALOperation {
    CreateCollection(CollectionConfig),
    DeleteCollection(CollectionId),
    InsertVector {
        collection: CollectionId,
        vector: Vector,
    },
    BatchInsert {
        collection: CollectionId,
        vectors: Vec<Vector>,
    },
    DeleteVector {
        collection: CollectionId,
        id: VectorId,
    },
}

/// WAL entry with metadata
#[derive(Debug, Serialize, Deserialize)]
struct WALEntry {
    id: Uuid,
    timestamp: u64,
    checksum: u32,
    operation: WALOperation,
}

/// Magic number for WAL entry boundaries (helps detect corruption)
const WAL_ENTRY_MAGIC: u32 = 0xDEADBEEF;

/// Calculate CRC32 checksum for data
fn calculate_checksum(data: &[u8]) -> u32 {
    let mut hasher = crc32fast::Hasher::new();
    hasher.update(data);
    hasher.finalize()
}

/// Write-Ahead Log for durability with batching
pub struct WriteAheadLog {
    path: PathBuf,
    buffer: Arc<Mutex<Vec<u8>>>,
    write_file: Arc<Mutex<Option<File>>>,
}

impl WriteAheadLog {
    pub async fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Open file in append mode and keep it open for performance
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await?;

        Ok(Self {
            path,
            buffer: Arc::new(Mutex::new(Vec::with_capacity(1024 * 1024))), // 1MB buffer
            write_file: Arc::new(Mutex::new(Some(file))),
        })
    }
    
    /// Append an operation to the WAL (buffered, no sync on every append)
    pub async fn append(&self, operation: &WALOperation) -> Result<()> {
        // Serialize operation for checksum calculation
        let op_serialized = bincode::serialize(&operation)
            .map_err(|e| VectorDbError::Serialization(e.to_string()))?;

        let checksum = calculate_checksum(&op_serialized);

        let entry = WALEntry {
            id: Uuid::new_v4(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            checksum,
            operation: operation.clone(),
        };

        let serialized = bincode::serialize(&entry)
            .map_err(|e| VectorDbError::Serialization(e.to_string()))?;

        // Write format: [MAGIC 4 bytes][LENGTH 4 bytes][DATA]
        let length = serialized.len() as u32;

        // PERFORMANCE OPTIMIZATION: Buffer writes instead of immediate sync
        // This reduces disk I/O from N ops to 1 batch
        let should_flush = {
            let mut buffer = self.buffer.lock().await;
            buffer.extend_from_slice(&WAL_ENTRY_MAGIC.to_le_bytes());
            buffer.extend_from_slice(&length.to_le_bytes());
            buffer.extend_from_slice(&serialized);

            // Flush to disk when buffer exceeds threshold (256KB)
            buffer.len() > 256 * 1024
        };

        if should_flush {
            self.flush_internal().await?;
        }

        tracing::debug!("Buffered WAL entry: {:?}", entry.id);
        Ok(())
    }

    /// Internal flush method - writes buffer to disk
    async fn flush_internal(&self) -> Result<()> {
        let data_to_write = {
            let mut buffer = self.buffer.lock().await;
            if buffer.is_empty() {
                return Ok(());
            }
            let data = buffer.clone();
            buffer.clear();
            data
        };

        let mut file_guard = self.write_file.lock().await;
        if let Some(ref mut file) = *file_guard {
            file.write_all(&data_to_write).await?;
            file.sync_all().await?;
        }

        Ok(())
    }
    
    /// Read all entries from the WAL with corruption detection
    pub async fn read_all(&self) -> Result<Vec<WALOperation>> {
        // Flush any buffered writes before reading
        self.flush_internal().await?;

        let file = File::open(&self.path).await?;
        let mut reader = BufReader::new(file);
        let mut operations = Vec::new();
        let mut corrupted_count = 0;

        loop {
            // Read magic number
            let mut magic_bytes = [0u8; 4];
            match reader.read_exact(&mut magic_bytes).await {
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(e.into()),
            }

            let magic = u32::from_le_bytes(magic_bytes);
            if magic != WAL_ENTRY_MAGIC {
                tracing::warn!("Corrupted WAL entry detected (bad magic): expected {:x}, got {:x}", WAL_ENTRY_MAGIC, magic);
                corrupted_count += 1;
                // Try to skip to next valid magic number
                continue;
            }

            // Read length prefix
            let mut length_bytes = [0u8; 4];
            match reader.read_exact(&mut length_bytes).await {
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    tracing::warn!("Truncated WAL entry (length)");
                    break;
                }
                Err(e) => return Err(e.into()),
            }

            let length = u32::from_le_bytes(length_bytes) as usize;

            // Sanity check on length (prevent excessive memory allocation)
            if length > 100 * 1024 * 1024 {
                // 100MB max per entry
                tracing::warn!("Suspiciously large WAL entry length: {} bytes, skipping", length);
                corrupted_count += 1;
                continue;
            }

            // Read entry data
            let mut entry_data = vec![0u8; length];
            match reader.read_exact(&mut entry_data).await {
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    tracing::warn!("Truncated WAL entry (data)");
                    break;
                }
                Err(e) => return Err(e.into()),
            }

            // Deserialize entry
            let entry: WALEntry = match bincode::deserialize(&entry_data) {
                Ok(e) => e,
                Err(e) => {
                    tracing::warn!("Failed to deserialize WAL entry: {}", e);
                    corrupted_count += 1;
                    continue;
                }
            };

            // Verify checksum
            let op_serialized = bincode::serialize(&entry.operation)
                .map_err(|e| VectorDbError::Serialization(e.to_string()))?;
            let calculated_checksum = calculate_checksum(&op_serialized);

            if calculated_checksum != entry.checksum {
                tracing::warn!(
                    "Checksum mismatch for WAL entry {:?}: expected {}, got {}",
                    entry.id,
                    entry.checksum,
                    calculated_checksum
                );
                corrupted_count += 1;
                continue;
            }

            operations.push(entry.operation);
        }

        if corrupted_count > 0 {
            tracing::warn!("Skipped {} corrupted WAL entries during recovery", corrupted_count);
        }

        tracing::info!("Read {} valid operations from WAL", operations.len());
        Ok(operations)
    }
    
    /// Sync the WAL to disk (flush buffer)
    pub async fn sync(&self) -> Result<()> {
        self.flush_internal().await
    }

    /// Truncate the WAL (after successful checkpoint)
    pub async fn truncate(&mut self) -> Result<()> {
        // First flush any pending writes
        self.flush_internal().await?;

        // Close the current file
        {
            let mut file_guard = self.write_file.lock().await;
            *file_guard = None;
        }

        // Truncate the file
        let _file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&self.path)
            .await?;

        // Reopen for appending
        let new_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .await?;

        {
            let mut file_guard = self.write_file.lock().await;
            *file_guard = Some(new_file);
        }

        tracing::info!("Truncated WAL");
        Ok(())
    }
    
    /// Get WAL file size
    pub async fn size(&self) -> Result<u64> {
        let metadata = tokio::fs::metadata(&self.path).await?;
        Ok(metadata.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_wal_operations() {
        let temp_dir = tempdir().unwrap();
        let wal_path = temp_dir.path().join("test.wal");
        
        let mut wal = WriteAheadLog::new(&wal_path).await.unwrap();
        
        let config = CollectionConfig {
            name: "test".to_string(),
            dimension: 128,
            distance_metric: DistanceMetric::Cosine,
            vector_type: VectorType::Float32,
            index_config: IndexConfig::default(),
        };
        
        let op = WALOperation::CreateCollection(config);
        wal.append(&op).await.unwrap();
        
        let operations = wal.read_all().await.unwrap();
        assert_eq!(operations.len(), 1);
        
        match &operations[0] {
            WALOperation::CreateCollection(c) => {
                assert_eq!(c.name, "test");
            }
            _ => panic!("Unexpected operation type"),
        }
    }
}