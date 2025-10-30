use vectordb_common::{Result, VectorDbError};
use std::path::{Path, PathBuf};
use std::fs::{File, OpenOptions};
use memmap2::{MmapMut, MmapOptions};
use parking_lot::Mutex;

const INITIAL_SIZE: u64 = 1024 * 1024; // 1MB initial size
const GROWTH_FACTOR: f64 = 2.0;

/// Memory-mapped file storage with automatic growth
pub struct MMapStorage {
    path: PathBuf,
    file: Mutex<File>,
    mmap: Mutex<Option<MmapMut>>,
    size: Mutex<u64>,
    position: Mutex<u64>,
}

impl MMapStorage {
    pub async fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();

        // Create or open file
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(&path)?;

        // Get current size or initialize
        let current_size = file.metadata()?.len();
        let size = if current_size == 0 {
            file.set_len(INITIAL_SIZE)?;
            INITIAL_SIZE
        } else {
            current_size
        };

        // Create memory mapping
        let mmap = unsafe {
            MmapOptions::new()
                .len(size as usize)
                .map_mut(&file)?
        };

        // Calculate actual data position by scanning existing records
        // This is crucial for restart persistence - we need to know where data ends
        let data_position = Self::calculate_data_end(&mmap, current_size)?;

        tracing::debug!(
            "Opened storage file: size={}, data_position={}",
            current_size,
            data_position
        );

        Ok(Self {
            path,
            file: Mutex::new(file),
            mmap: Mutex::new(Some(mmap)),
            size: Mutex::new(size),
            position: Mutex::new(data_position),
        })
    }

    /// Calculate where actual data ends by scanning length-prefixed records
    fn calculate_data_end(mmap: &MmapMut, file_size: u64) -> Result<u64> {
        let mut position: u64 = 0;
        let mut record_count = 0;

        // Scan through all records to find where data actually ends
        while position + 4 <= file_size {
            // Read length prefix (4 bytes, u32 little-endian)
            let length_bytes = &mmap[position as usize..(position + 4) as usize];
            let length = u32::from_le_bytes([
                length_bytes[0],
                length_bytes[1],
                length_bytes[2],
                length_bytes[3],
            ]) as u64;

            // Check for end of data (zero length or invalid)
            if length == 0 || position + 4 + length > file_size {
                break;
            }

            // Move to next record
            position += 4 + length;
            record_count += 1;
        }

        tracing::debug!(
            "Scanned storage: found {} records, data ends at position {}",
            record_count,
            position
        );

        Ok(position)
    }
    
    /// Append data to the storage
    pub async fn append(&self, data: &[u8]) -> Result<u64> {
        let (current_position, need_grow) = {
            let position = self.position.lock();
            let size = self.size.lock();
            (*position, *position + data.len() as u64 > *size)
        };
        
        // Check if we need to grow the file
        if need_grow {
            self.grow((current_position + data.len() as u64) * 2).await?;
        }
        
        // Write data to memory map and update position
        let old_position = {
            let mut position = self.position.lock();
            let mut mmap_guard = self.mmap.lock();
            if let Some(mmap) = mmap_guard.as_mut() {
                let start = *position as usize;
                let end = start + data.len();
                mmap[start..end].copy_from_slice(data);
            }
            
            let old_position = *position;
            *position += data.len() as u64;
            old_position
        };
        
        Ok(old_position)
    }
    
    /// Read data at a specific offset
    pub async fn read(&self, offset: u64, length: usize) -> Result<Vec<u8>> {
        let mmap_guard = self.mmap.lock();
        if let Some(mmap) = mmap_guard.as_ref() {
            let start = offset as usize;
            let end = start + length;
            
            if end > mmap.len() {
                return Err(VectorDbError::StorageError {
                    message: "Read beyond file boundary".to_string(),
                });
            }
            
            Ok(mmap[start..end].to_vec())
        } else {
            Err(VectorDbError::StorageError {
                message: "Memory map not available".to_string(),
            })
        }
    }
    
    /// Get current file size
    pub async fn size(&self) -> Result<u64> {
        Ok(*self.size.lock())
    }
    
    /// Get current write position
    pub async fn position(&self) -> Result<u64> {
        Ok(*self.position.lock())
    }
    
    /// Sync data to disk
    pub async fn sync(&self) -> Result<()> {
        let mmap_guard = self.mmap.lock();
        if let Some(mmap) = mmap_guard.as_ref() {
            mmap.flush()?;
        }
        
        self.file.lock().sync_all()?;
        Ok(())
    }
    
    /// Grow the file to a new size
    async fn grow(&self, new_size: u64) -> Result<()> {
        let file = self.file.lock();
        let mut mmap_guard = self.mmap.lock();
        let mut size = self.size.lock();
        
        // Drop current memory map
        *mmap_guard = None;
        
        // Resize file
        file.set_len(new_size)?;
        file.sync_all()?;
        
        // Create new memory map
        let new_mmap = unsafe {
            MmapOptions::new()
                .len(new_size as usize)
                .map_mut(&*file)?
        };
        
        *mmap_guard = Some(new_mmap);
        *size = new_size;
        
        tracing::debug!("Grew storage file to {} bytes", new_size);
        Ok(())
    }
    
    /// Iterate over all records in the storage
    pub async fn iter(&self) -> Result<StorageIterator> {
        Ok(StorageIterator {
            storage: self,
            position: 0,
        })
    }
}

/// Iterator over storage records
pub struct StorageIterator<'a> {
    storage: &'a MMapStorage,
    position: u64,
}

impl<'a> StorageIterator<'a> {
    /// Get the next record
    pub async fn next(&mut self) -> Result<Option<Vec<u8>>> {
        let current_size = self.storage.position().await?;
        
        if self.position >= current_size {
            return Ok(None);
        }
        
        // Read length prefix (4 bytes)
        if self.position + 4 > current_size {
            return Ok(None);
        }
        
        let length_bytes = self.storage.read(self.position, 4).await?;
        let length = u32::from_le_bytes([
            length_bytes[0],
            length_bytes[1], 
            length_bytes[2],
            length_bytes[3],
        ]) as u64;
        
        if self.position + 4 + length > current_size {
            return Ok(None);
        }
        
        // Read data
        let data = self.storage.read(self.position + 4, length as usize).await?;
        self.position += 4 + length;
        
        Ok(Some(data))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_mmap_storage() {
        let temp_dir = tempdir().unwrap();
        let storage_path = temp_dir.path().join("test.bin");
        
        let storage = MMapStorage::new(&storage_path).await.unwrap();
        
        let data = b"Hello, World!";
        let offset = storage.append(data).await.unwrap();
        assert_eq!(offset, 0);
        
        let read_data = storage.read(0, data.len()).await.unwrap();
        assert_eq!(read_data, data);
        
        let size = storage.size().await.unwrap();
        assert_eq!(size, INITIAL_SIZE);
    }
    
    #[tokio::test]
    async fn test_storage_growth() {
        let temp_dir = tempdir().unwrap();
        let storage_path = temp_dir.path().join("test.bin");
        
        let storage = MMapStorage::new(&storage_path).await.unwrap();
        
        // Write data larger than initial size
        let large_data = vec![42u8; (INITIAL_SIZE + 1000) as usize];
        storage.append(&large_data).await.unwrap();
        
        let size = storage.size().await.unwrap();
        assert!(size > INITIAL_SIZE);
    }
}