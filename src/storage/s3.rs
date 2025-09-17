use async_trait::async_trait;
use tokio::io::AsyncRead;

use crate::{
    errors::{AppError, Result},
    storage::Storage,
};

pub struct S3Storage {
    bucket: String,
    region: String,
    endpoint: Option<String>,
    access_key: String,
    secret_key: String,
}

impl S3Storage {
    pub fn new(
        bucket: String,
        region: String,
        endpoint: Option<String>,
        access_key: String,
        secret_key: String,
    ) -> Self {
        Self {
            bucket,
            region,
            endpoint,
            access_key,
            secret_key,
        }
    }
}

#[async_trait]
impl Storage for S3Storage {
    async fn store<R>(&self, _path: &str, _reader: R) -> Result<()>
    where
        R: AsyncRead + Send + Unpin,
    {
        // TODO: Implement S3 storage using aws-sdk-s3 or similar
        Err(AppError::Storage("S3 storage not yet implemented".to_string()))
    }

    async fn retrieve(&self, _path: &str) -> Result<Box<dyn AsyncRead + Send + Unpin>> {
        Err(AppError::Storage("S3 storage not yet implemented".to_string()))
    }

    async fn delete(&self, _path: &str) -> Result<()> {
        Err(AppError::Storage("S3 storage not yet implemented".to_string()))
    }

    async fn exists(&self, _path: &str) -> Result<bool> {
        Err(AppError::Storage("S3 storage not yet implemented".to_string()))
    }

    async fn size(&self, _path: &str) -> Result<u64> {
        Err(AppError::Storage("S3 storage not yet implemented".to_string()))
    }
}
