use axum::{
    extract::{Multipart, Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use tokio::io::AsyncReadExt;
use uuid::Uuid;

use crate::{
    database::queries::{ImageQueries, UsageQueries},
    errors::{AppError, Result},
    handlers::AppState,
    middleware::auth::AuthenticatedApiKey,
    models::{Image, ImageMetadata, ImageVariant, ImageVariants, UploadResponse, Visibility},
    services::image_processor::ImageProcessor,
    storage::Storage,
};

#[derive(Debug, Deserialize)]
pub struct ImageQuery {
    pub w: Option<u32>,
    pub h: Option<u32>,
    pub format: Option<String>,
    pub download: Option<bool>,
}

pub async fn upload_image(
    State(state): State<AppState>,
    api_key: AuthenticatedApiKey,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>> {
    let mut file_data: Option<Vec<u8>> = None;
    let mut filename: Option<String> = None;
    let mut resize_params: Vec<u32> = Vec::new();
    let mut visibility = Visibility::Public;

    // Parse multipart form data
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        AppError::Validation(format!("Failed to parse multipart data: {}", e))
    })? {
        let field_name = field.name().unwrap_or("").to_string();

        match field_name.as_str() {
            "file" => {
                let data = field.bytes().await.map_err(|e| {
                    AppError::Validation(format!("Failed to read file data: {}", e))
                })?;
                file_data = Some(data.to_vec());
            }
            "filename" => {
                filename = Some(field.text().await.map_err(|e| {
                    AppError::Validation(format!("Failed to read filename: {}", e))
                })?);
            }
            "resize" => {
                let resize_str = field.text().await.map_err(|e| {
                    AppError::Validation(format!("Failed to read resize parameter: {}", e))
                })?;
                if let Ok(size) = resize_str.parse::<u32>() {
                    resize_params.push(size);
                }
            }
            "visibility" => {
                let vis_str = field.text().await.map_err(|e| {
                    AppError::Validation(format!("Failed to read visibility: {}", e))
                })?;
                visibility = match vis_str.as_str() {
                    "private" => Visibility::Private,
                    _ => Visibility::Public,
                };
            }
            _ => {} // Ignore unknown fields
        }
    }

    let file_data = file_data.ok_or_else(|| {
        AppError::Validation("No file provided".to_string())
    })?;

    // Validate file size against API key limits
    let limits: serde_json::Value = api_key.limits;
    let max_size = limits
        .get("max_image_size_bytes")
        .and_then(|v| v.as_u64())
        .unwrap_or(20 * 1024 * 1024); // Default 20MB

    if file_data.len() as u64 > max_size {
        return Err(AppError::Validation(format!(
            "File size {} bytes exceeds limit of {} bytes",
            file_data.len(),
            max_size
        )));
    }

    // Validate MIME type and extract image metadata
    let (mime_type, width, height) = validate_and_extract_metadata(&file_data)?;

    // Compute SHA256 hash for deduplication
    let mut hasher = Sha256::new();
    hasher.update(&file_data);
    let sha256 = format!("{:x}", hasher.finalize());

    // Check for existing image with same hash
    if let Some(existing_image) = ImageQueries::find_by_sha256(state.database.pool(), &sha256).await? {
        // Return existing image metadata
        let variants = parse_image_variants(&existing_image.variants)?;
        
        let response = UploadResponse {
            id: existing_image.id,
            url: format!("/v1/images/{}", existing_image.id),
            variants,
            metadata: ImageMetadata {
                width: existing_image.width,
                height: existing_image.height,
                size_bytes: existing_image.orig_size_bytes,
                mime: existing_image.mime,
                sha256: existing_image.sha256,
                created_at: existing_image.created_at,
            },
        };

        // Update usage counter for duplicate
        UsageQueries::update_usage(
            state.database.pool(),
            api_key.id,
            1, // request count
            0, // bytes served (no new storage)
            0, // uploads (duplicate)
        ).await?;

        return Ok(Json(json!({
            "message": "Image already exists (deduplicated)",
            "data": response
        })));
    }

    // Generate storage path
    let image_id = Uuid::new_v4();
    let extension = get_extension_from_mime(&mime_type);
    let storage_path = generate_storage_path(&image_id, &extension);

    // Store original image
    let storage = create_storage_backend(&state.config.storage)?;
    storage.store(&storage_path, &file_data[..]).await.map_err(|e| {
        AppError::Storage(format!("Failed to store image: {}", e))
    })?;

    // Create image record in database
    let is_public = matches!(visibility, Visibility::Public);
    let image = ImageQueries::create_image(
        state.database.pool(),
        api_key.owner_id,
        &sha256,
        &mime_type,
        file_data.len() as i64,
        width,
        height,
        &storage_path,
        is_public,
    ).await?;

    // Queue image processing job
    let processor = ImageProcessor::new(&state.config.image_processing);
    let processing_job = processor.queue_processing_job(
        image.id,
        &file_data,
        &resize_params,
        &state.config.image_processing.thumbnail_sizes,
    ).await?;

    // For now, return basic variants (processing will update these)
    let variants = ImageVariants {
        original: ImageVariant {
            width: width as u32,
            height: height as u32,
            format: mime_type.clone(),
            size_bytes: file_data.len() as u64,
            url: format!("/v1/images/{}", image.id),
        },
        webp: None,
        avif: None,
        thumbnails: Vec::new(),
    };

    let response = UploadResponse {
        id: image.id,
        url: format!("/v1/images/{}", image.id),
        variants,
        metadata: ImageMetadata {
            width: image.width,
            height: image.height,
            size_bytes: image.orig_size_bytes,
            mime: image.mime,
            sha256: image.sha256,
            created_at: image.created_at,
        },
    };

    // Update usage counter
    UsageQueries::update_usage(
        state.database.pool(),
        api_key.id,
        1, // request count
        0, // bytes served
        1, // uploads
    ).await?;

    Ok(Json(json!({
        "message": "Image uploaded successfully",
        "data": response,
        "processing_job_id": processing_job.id
    })))
}

pub async fn get_image(
    State(state): State<AppState>,
    api_key: AuthenticatedApiKey,
    Path(image_id): Path<Uuid>,
    Query(query): Query<ImageQuery>,
) -> Result<Response> {
    // Find image
    let image = ImageQueries::find_by_id(state.database.pool(), image_id)
        .await?
        .ok_or(AppError::NotFound)?;

    // Check access permissions
    if !image.is_public && image.owner_id != api_key.owner_id {
        return Err(AppError::Forbidden);
    }

    // Parse variants
    let variants = parse_image_variants(&image.variants)?;

    // Determine which variant to serve
    let variant_url = if let Some(width) = query.w {
        // Find closest thumbnail size
        let closest_thumbnail = variants.thumbnails
            .iter()
            .min_by_key(|t| (t.width as i32 - width as i32).abs())
            .map(|t| t.url.clone())
            .unwrap_or_else(|| format!("/v1/images/{}", image.id));
        closest_thumbnail
    } else if let Some(format) = query.format {
        match format.as_str() {
            "webp" => variants.webp.map(|v| v.url).unwrap_or_else(|| format!("/v1/images/{}", image.id)),
            "avif" => variants.avif.map(|v| v.url).unwrap_or_else(|| format!("/v1/images/{}", image.id)),
            _ => format!("/v1/images/{}", image.id),
        }
    } else {
        format!("/v1/images/{}", image.id)
    };

    // For now, return a redirect to the storage URL
    // In production, you'd serve the actual file or redirect to CDN
    let response = axum::response::Redirect::temporary(&variant_url);

    // Update usage counter
    UsageQueries::update_usage(
        state.database.pool(),
        api_key.id,
        1, // request count
        image.orig_size_bytes, // bytes served
        0, // uploads
    ).await?;

    Ok(response.into_response())
}

pub async fn delete_image(
    State(state): State<AppState>,
    api_key: AuthenticatedApiKey,
    Path(image_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    // Find image
    let image = ImageQueries::find_by_id(state.database.pool(), image_id)
        .await?
        .ok_or(AppError::NotFound)?;

    // Check permissions - only owner can delete
    if image.owner_id != api_key.owner_id {
        return Err(AppError::Forbidden);
    }

    // Soft delete the image
    ImageQueries::soft_delete(state.database.pool(), image_id).await?;

    // TODO: Queue background job to remove from storage

    Ok(Json(json!({
        "message": "Image deleted successfully"
    })))
}

pub async fn get_metadata(
    State(state): State<AppState>,
    api_key: AuthenticatedApiKey,
    Path(image_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    // Find image
    let image = ImageQueries::find_by_id(state.database.pool(), image_id)
        .await?
        .ok_or(AppError::NotFound)?;

    // Check access permissions
    if !image.is_public && image.owner_id != api_key.owner_id {
        return Err(AppError::Forbidden);
    }

    let variants = parse_image_variants(&image.variants)?;

    let metadata = ImageMetadata {
        width: image.width,
        height: image.height,
        size_bytes: image.orig_size_bytes,
        mime: image.mime,
        sha256: image.sha256,
        created_at: image.created_at,
    };

    Ok(Json(json!({
        "data": {
            "metadata": metadata,
            "variants": variants,
            "is_public": image.is_public
        }
    })))
}

// Helper functions

fn validate_and_extract_metadata(data: &[u8]) -> Result<(String, i32, i32)> {
    // Check file signature (magic bytes)
    if data.len() < 8 {
        return Err(AppError::Validation("File too small".to_string()));
    }

    let mime_type = match &data[0..8] {
        [0xFF, 0xD8, 0xFF, ..] => "image/jpeg",
        [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A] => "image/png",
        [0x52, 0x49, 0x46, 0x46, _, _, _, _] if &data[8..12] == b"WEBP" => "image/webp",
        [_, _, _, _, 0x66, 0x74, 0x79, 0x70] => {
            // Check for AVIF
            if data.len() > 20 && &data[16..20] == b"avif" {
                "image/avif"
            } else {
                return Err(AppError::Validation("Unsupported image format".to_string()));
            }
        }
        [0x47, 0x49, 0x46, 0x38, ..] => "image/gif",
        _ => return Err(AppError::Validation("Unsupported image format".to_string())),
    };

    // Use image crate to get dimensions
    let img = image::load_from_memory(data)
        .map_err(|e| AppError::Validation(format!("Invalid image data: {}", e)))?;

    Ok((mime_type.to_string(), img.width() as i32, img.height() as i32))
}

fn get_extension_from_mime(mime_type: &str) -> String {
    match mime_type {
        "image/jpeg" => "jpg",
        "image/png" => "png",
        "image/webp" => "webp",
        "image/avif" => "avif",
        "image/gif" => "gif",
        _ => "bin",
    }.to_string()
}

fn generate_storage_path(image_id: &Uuid, extension: &str) -> String {
    let now = chrono::Utc::now();
    format!(
        "images/{}/{:02}/{}.{}",
        now.year(),
        now.month(),
        image_id,
        extension
    )
}

fn parse_image_variants(variants_json: &serde_json::Value) -> Result<ImageVariants> {
    // For now, return empty variants - this will be populated by the image processor
    Ok(ImageVariants {
        original: ImageVariant {
            width: 0,
            height: 0,
            format: "unknown".to_string(),
            size_bytes: 0,
            url: "".to_string(),
        },
        webp: None,
        avif: None,
        thumbnails: Vec::new(),
    })
}

fn create_storage_backend(config: &crate::config::StorageConfig) -> Result<Box<dyn Storage>> {
    match config.storage_type {
        crate::config::StorageType::Local => {
            let path = config.local_path.as_ref()
                .ok_or_else(|| AppError::Storage("Local storage path not configured".to_string()))?;
            Ok(Box::new(crate::storage::local::LocalStorage::new(path)?))
        }
        crate::config::StorageType::S3 => {
            // TODO: Implement S3 storage
            Err(AppError::Storage("S3 storage not yet implemented".to_string()))
        }
    }
}
