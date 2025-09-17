use async_trait::async_trait;
use std::path::Path;
use tokio::io::{AsyncRead, AsyncWrite};
use crate::errors::Result;
use crate::config::Config;

pub mod local;
pub mod s3;

#[async_trait]
pub trait Storage: Send + Sync {
    async fn store<R>(&self, path: &str, reader: R) -> Result<()>
    where
        R: AsyncRead + Send + Unpin;

    async fn retrieve(&self, path: &str) -> Result<Box<dyn AsyncRead + Send + Unpin>>;

    async fn delete(&self, path: &str) -> Result<()>;

    async fn exists(&self, path: &str) -> Result<bool>;

    async fn size(&self, path: &str) -> Result<u64>;
}

pub fn create_storage(config: &Config) -> Result<Box<dyn Storage>> {
    match config.storage.storage_type.as_str() {
        "local" => {
            let storage = local::LocalStorage::new(&config.storage.local_path)?;
            Ok(Box::new(storage))
        }
        "s3" => {
            let storage = s3::S3Storage::new(
                &config.storage.s3_endpoint.as_ref().unwrap(),
                &config.storage.s3_bucket.as_ref().unwrap(),
                &config.storage.s3_access_key.as_ref().unwrap(),
                &config.storage.s3_secret_key.as_ref().unwrap(),
                config.storage.s3_region.as_deref(),
            ).await?;
            Ok(Box::new(storage))
        }
        _ => Err(crate::errors::AppError::Config(
            format!("Unsupported storage type: {}", config.storage.storage_type)
        )),
    }
}
