use std::path::{Path, PathBuf};
use std::fs;
use uuid::Uuid;
use anyhow::Result;
use crate::error::AppError;

pub fn create_upload_directory(base_path: &str) -> Result<()> {
    let path = Path::new(base_path);
    if !path.exists() {
        fs::create_dir_all(path)?;
    }
    Ok(())
}

pub fn generate_file_path(base_dir: &str, user_id: Uuid, filename: &str) -> PathBuf {
    let user_dir = user_id.to_string();
    let date_dir = chrono::Utc::now().format("%Y/%m/%d").to_string();
    
    Path::new(base_dir)
        .join(user_dir)
        .join(date_dir)
        .join(filename)
}

pub fn ensure_directory_exists(file_path: &Path) -> Result<()> {
    if let Some(parent) = file_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }
    Ok(())
}

pub fn validate_mime_type(mime_type: &str, allowed_types: &[String]) -> Result<()> {
    if !allowed_types.contains(&mime_type.to_string()) {
        return Err(AppError::InvalidFileFormat.into());
    }
    Ok(())
}

pub fn get_file_extension(mime_type: &str) -> &'static str {
    match mime_type {
        "image/jpeg" => "jpg",
        "image/png" => "png",
        "image/webp" => "webp",
        "image/gif" => "gif",
        _ => "bin",
    }
}

pub async fn save_file(file_path: &Path, data: &[u8]) -> Result<()> {
    ensure_directory_exists(file_path)?;
    tokio::fs::write(file_path, data).await?;
    Ok(())
}

pub async fn delete_file(file_path: &Path) -> Result<()> {
    if file_path.exists() {
        tokio::fs::remove_file(file_path).await?;
    }
    Ok(())
}
