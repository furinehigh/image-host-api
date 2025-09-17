use axum::{
    extract::{Multipart, State, Request},
    response::Json,
    http::StatusCode,
};
use uuid::Uuid;
use std::path::Path;
use crate::{
    handlers::AppState,
    models::{Image, ImageResponse},
    error::{AppError, Result},
    utils::{crypto, file},
    services::{
        image_processor::{ImageProcessor, ProcessingOptions},
        virus_scanner::VirusScanner,
    },
    middleware::{
        auth::{get_authenticated_user, AuthenticatedUser},
        rate_limit::EndpointRateLimiter,
    },
};

pub async fn upload_image(
    State(state): State<AppState>,
    request: Request,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<ImageResponse>)> {
    let user = get_authenticated_user(&request)?;
    let user_id = user.user_id;

    // Check upload-specific rate limit
    if !EndpointRateLimiter::check_upload_limit(&state, &user_id.to_string()).await? {
        return Err(AppError::RateLimitExceeded);
    }

    let mut file_data: Option<Vec<u8>> = None;
    let mut original_filename: Option<String> = None;
    let mut mime_type: Option<String> = None;

    // Process multipart form data
    while let Some(field) = multipart.next_field().await.map_err(|_| AppError::InvalidFileFormat)? {
        let name = field.name().unwrap_or("").to_string();
        
        if name == "file" {
            original_filename = field.file_name().map(|s| s.to_string());
            mime_type = field.content_type().map(|s| s.to_string());
            file_data = Some(field.bytes().await.map_err(|_| AppError::InvalidFileFormat)?.to_vec());
        }
    }

    let file_data = file_data.ok_or(AppError::InvalidFileFormat)?;
    let original_filename = original_filename.ok_or(AppError::InvalidFileFormat)?;
    let mime_type = mime_type.ok_or(AppError::InvalidFileFormat)?;

    // Validate file size
    if file_data.len() > state.config.max_file_size {
        return Err(AppError::FileTooLarge);
    }

    // Validate MIME type
    file::validate_mime_type(&mime_type, &state.config.allowed_mime_types)?;

    ImageProcessor::validate_image(&file_data, state.config.max_image_dimension, state.config.max_file_size)?;

    if state.config.virus_scan_enabled {
        let scanner = VirusScanner::new(state.config.virus_scan_url.clone(), true);
        scanner.scan_file(&file_data, &original_filename).await?;
    }

    // Check user quota
    let user = state.database.get_user_by_id(user_id).await?
        .ok_or(AppError::Unauthorized)?;
    
    let current_usage = state.database.get_user_storage_usage(user_id).await?;
    if current_usage + file_data.len() as i64 > user.quota_bytes {
        return Err(AppError::QuotaExceeded);
    }

    // Calculate SHA256 hash for deduplication
    let sha256_hash = crypto::calculate_sha256(&file_data);
    
    // Check if image already exists (deduplication)
    if let Some(existing_image) = state.database.get_image_by_hash(&sha256_hash).await? {
        let response = ImageResponse {
            id: existing_image.id,
            filename: existing_image.filename,
            mime_type: existing_image.mime_type,
            file_size: existing_image.file_size,
            width: existing_image.width,
            height: existing_image.height,
            url: format!("/api/v1/images/{}", existing_image.id),
            created_at: existing_image.created_at,
        };
        
        return Ok((StatusCode::OK, Json(response)));
    }

    let processing_options = ProcessingOptions {
        optimize: true,
        strip_metadata: true,
        progressive: true,
        quality: 85,
    };

    let optimized_data = ImageProcessor::optimize_image(&file_data, &processing_options)?;
    let final_data = if optimized_data.len() < file_data.len() {
        optimized_data
    } else {
        file_data // Use original if optimization didn't help
    };

    // Get image info from processed data
    let image_info = ImageProcessor::get_image_info(&final_data)?;

    // Generate unique filename and storage path
    let image_id = Uuid::new_v4();
    let file_extension = file::get_file_extension(&mime_type);
    let filename = format!("{}.{}", image_id, file_extension);
    let storage_path = file::generate_file_path(&state.config.upload_dir, user_id, &filename);

    // Save file to disk
    file::save_file(&storage_path, &final_data).await?;

    let thumbnail_data = ImageProcessor::generate_thumbnail(&final_data, 256)?;
    let thumbnail_path = storage_path.with_extension(format!("thumb.{}", file_extension));
    file::save_file(&thumbnail_path, &thumbnail_data).await?;

    // Create image record in database
    let image = Image {
        id: image_id,
        user_id,
        filename: filename.clone(),
        original_filename,
        mime_type: mime_type.clone(),
        file_size: final_data.len() as i64,
        width: image_info.width as i32,
        height: image_info.height as i32,
        sha256_hash: crypto::calculate_sha256(&final_data), // Hash of processed image
        storage_path: storage_path.to_string_lossy().to_string(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let created_image = state.database.create_image(&image).await?;

    // Update user's storage usage
    let new_usage = current_usage + final_data.len() as i64;
    state.database.update_user_quota(user_id, new_usage).await?;

    // Cache image metadata
    let cache_data = crate::services::redis::ImageCacheData {
        filename: created_image.filename.clone(),
        mime_type: created_image.mime_type.clone(),
        file_size: created_image.file_size,
        width: created_image.width,
        height: created_image.height,
        storage_path: created_image.storage_path.clone(),
    };
    
    state.redis.cache_image_metadata(
        &created_image.id.to_string(),
        &cache_data,
        std::time::Duration::from_secs(3600)
    ).await.ok();

    let response = ImageResponse {
        id: created_image.id,
        filename: created_image.filename,
        mime_type: created_image.mime_type,
        file_size: created_image.file_size,
        width: created_image.width,
        height: created_image.height,
        url: format!("/api/v1/images/{}", created_image.id),
        created_at: created_image.created_at,
    };

    Ok((StatusCode::CREATED, Json(response)))
}
