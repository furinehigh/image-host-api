use async_trait::async_trait;
use std::path::{Path, PathBuf};
use tokio::{fs, io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt}};
use tokio_util::io::ReaderStream;

use crate::{
    errors::{AppError, Result},
    storage::Storage,
};

pub struct LocalStorage {
    base_path: PathBuf,
}

impl LocalStorage {
    pub fn new<P: AsRef<Path>>(base_path: P) -> Result<Self> {
        let base_path = base_path.as_ref().to_path_buf();
        
        // Create base directory if it doesn't exist
        std::fs::create_dir_all(&base_path)
            .map_err(|e| AppError::Storage(format!("Failed to create storage directory: {}", e)))?;

        Ok(Self { base_path })
    }

    fn get_full_path(&self, path: &str) -> PathBuf {
        self.base_path.join(path)
    }
}

#[async_trait]
impl Storage for LocalStorage {
    async fn store<R>(&self, path: &str, mut reader: R) -> Result<()>
    where
        R: AsyncRead + Send + Unpin,
    {
        let full_path = self.get_full_path(path);
        
        // Create parent directories if they don't exist
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).await
                .map_err(|e| AppError::Storage(format!("Failed to create directory: {}", e)))?;
        }

        // Read data from reader
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer).await
            .map_err(|e| AppError::Storage(format!("Failed to read data: {}", e)))?;

        // Write to file
        fs::write(&full_path, buffer).await
            .map_err(|e| AppError::Storage(format!("Failed to write file: {}", e)))?;

        Ok(())
    }

    async fn retrieve(&self, path: &str) -> Result<Box<dyn AsyncRead + Send + Unpin>> {
        let full_path = self.get_full_path(path);
        
        let file = fs::File::open(&full_path).await
            .map_err(|e| AppError::Storage(format!("Failed to open file: {}", e)))?;

        Ok(Box::new(file))
    }

    async fn delete(&self, path: &str) -> Result<()> {
        let full_path = self.get_full_path(path);
        
        fs::remove_file(&full_path).await
            .map_err(|e| AppError::Storage(format!("Failed to delete file: {}", e)))?;

        Ok(())
    }

    async fn exists(&self, path: &str) -> Result<bool> {
        let full_path = self.get_full_path(path);
        Ok(full_path.exists())
    }

    async fn size(&self, path: &str) -> Result<u64> {
        let full_path = self.get_full_path(path);
        
        let metadata = fs::metadata(&full_path).await
            .map_err(|e| AppError::Storage(format!("Failed to get file metadata: {}", e)))?;

        Ok(metadata.len())
    }
}

// Helper implementation for storing byte slices
impl LocalStorage {
    pub async fn store_bytes(&self, path: &str, data: &[u8]) -> Result<()> {
        let full_path = self.get_full_path(path);
        
        // Create parent directories if they don't exist
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).await
                .map_err(|e| AppError::Storage(format!("Failed to create directory: {}", e)))?;
        }

        fs::write(&full_path, data).await
            .map_err(|e| AppError::Storage(format!("Failed to write file: {}", e)))?;

        Ok(())
    }

    pub async fn retrieve_bytes(&self, path: &str) -> Result<Vec<u8>> {
        let full_path = self.get_full_path(path);
        
        fs::read(&full_path).await
            .map_err(|e| AppError::Storage(format!("Failed to read file: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_local_storage_operations() {
        let temp_dir = tempdir().unwrap();
        let storage = LocalStorage::new(temp_dir.path()).unwrap();

        let test_data = b"Hello, World!";
        let test_path = "test/file.txt";

        // Test store
        storage.store_bytes(test_path, test_data).await.unwrap();

        // Test exists
        assert!(storage.exists(test_path).await.unwrap());

        // Test retrieve
        let retrieved_data = storage.retrieve_bytes(test_path).await.unwrap();
        assert_eq!(retrieved_data, test_data);

        // Test size
        let size = storage.size(test_path).await.unwrap();
        assert_eq!(size, test_data.len() as u64);

        // Test delete
        storage.delete(test_path).await.unwrap();
        assert!(!storage.exists(test_path).await.unwrap());
    }
}
