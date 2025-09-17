use axum::{
    extract::{Path, Query, State, Request},
    response::{Response, Json},
    http::{StatusCode, header},
    body::Body,
};
use uuid::Uuid;
use serde::Deserialize;
use std::collections::HashMap;
use crate::{
    handlers::AppState,
    models::{ImageTransformParams, ImageResponse},
    error::{AppError, Result},
    services::image_processor::ImageProcessor,
    middleware::auth::{get_authenticated_user, AuthenticatedUser},
};

#[derive(Deserialize)]
pub struct TransformQuery {
    width: Option<u32>,
    height: Option<u32>,
    quality: Option<u8>,
    format: Option<String>,
}

pub async fn get_image(
    State(state): State<AppState>,
    Path(image_id): Path<Uuid>,
) -> Result<Response<Body>> {
    // Try to get from cache first
    if let Ok(Some(cached_data)) = state.redis.get_cached_image_metadata(&image_id.to_string()).await {
        let file_data = tokio::fs::read(&cached_data.storage_path).await?;
        
        let response = Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, cached_data.mime_type)
            .header(header::CONTENT_LENGTH, cached_data.file_size.to_string())
            .header(header::CACHE_CONTROL, "public, max-age=31536000") // 1 year
            .body(Body::from(file_data))?;

        return Ok(response);
    }

    // Get from database
    let image = state.database.get_image_by_id(image_id).await?
        .ok_or(AppError::FileNotFound)?;

    // Read file from disk
    let file_data = tokio::fs::read(&image.storage_path).await
        .map_err(|_| AppError::FileNotFound)?;

    // Cache the metadata for future requests
    let cache_data = crate::services::redis::ImageCacheData {
        filename: image.filename,
        mime_type: image.mime_type.clone(),
        file_size: image.file_size,
        width: image.width,
        height: image.height,
        storage_path: image.storage_path,
    };
    
    state.redis.cache_image_metadata(
        &image_id.to_string(),
        &cache_data,
        std::time::Duration::from_secs(3600)
    ).await.ok();

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, image.mime_type)
        .header(header::CONTENT_LENGTH, image.file_size.to_string())
        .header(header::CACHE_CONTROL, "public, max-age=31536000")
        .body(Body::from(file_data))?;

    Ok(response)
}

pub async fn transform_image(
    State(state): State<AppState>,
    Path(image_id): Path<Uuid>,
    Query(params): Query<TransformQuery>,
) -> Result<Response<Body>> {
    // Generate cache key for transformed image
    let cache_key = format!(
        "{}:{}:{}:{}:{}",
        image_id,
        params.width.unwrap_or(0),
        params.height.unwrap_or(0),
        params.quality.unwrap_or(85),
        params.format.as_deref().unwrap_or("original")
    );

    // Check cache first
    if let Ok(Some(cached_data)) = state.redis.get_cached_transformed_image(&cache_key).await {
        let content_type = match params.format.as_deref() {
            Some("webp") => "image/webp",
            Some("png") => "image/png",
            Some("jpeg") | Some("jpg") => "image/jpeg",
            _ => "image/jpeg", // default
        };

        let response = Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, content_type)
            .header(header::CONTENT_LENGTH, cached_data.len().to_string())
            .header(header::CACHE_CONTROL, "public, max-age=86400") // 1 day
            .body(Body::from(cached_data))?;
            
        return Ok(response);
    }

    // Get original image
    let image = state.database.get_image_by_id(image_id).await?
        .ok_or(AppError::FileNotFound)?;

    // Read original file
    let original_data = tokio::fs::read(&image.storage_path).await
        .map_err(|_| AppError::FileNotFound)?;

    // Transform image
    let transform_params = ImageTransformParams {
        width: params.width,
        height: params.height,
        quality: params.quality,
        format: params.format.clone(),
    };

    let transformed_data = ImageProcessor::transform_image(&original_data, &transform_params)?;

    // Cache transformed image
    state.redis.cache_transformed_image(
        &cache_key,
        &transformed_data,
        std::time::Duration::from_secs(86400) // 1 day
    ).await.ok();

    let content_type = match params.format.as_deref() {
        Some("webp") => "image/webp",
        Some("png") => "image/png",
        Some("jpeg") | Some("jpg") => "image/jpeg",
        _ => &image.mime_type,
    };

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CONTENT_LENGTH, transformed_data.len().to_string())
        .header(header::CACHE_CONTROL, "public, max-age=86400")
        .body(Body::from(transformed_data))?;

    Ok(response)
}

pub async fn delete_image(
    State(state): State<AppState>,
    Path(image_id): Path<Uuid>,
    request: Request,
) -> Result<StatusCode> {
    let user = get_authenticated_user(&request)?;
    let user_id = user.user_id;

    // Get image to verify ownership and get file path
    let image = state.database.get_image_by_id(image_id).await?
        .ok_or(AppError::FileNotFound)?;

    // Verify ownership
    if image.user_id != user_id {
        return Err(AppError::Unauthorized);
    }

    // Delete from database
    let deleted = state.database.delete_image(image_id, user_id).await?;
    if !deleted {
        return Err(AppError::FileNotFound);
    }

    // Delete file from disk
    let file_path = std::path::Path::new(&image.storage_path);
    crate::utils::file::delete_file(file_path).await.ok(); // Don't fail if file doesn't exist

    // Update user's storage usage
    let current_usage = state.database.get_user_storage_usage(user_id).await?;
    let new_usage = (current_usage - image.file_size).max(0);
    state.database.update_user_quota(user_id, new_usage).await?;

    // Clear cache
    state.redis.get_connection()?.del::<_, ()>(format!("image:meta:{}", image_id)).ok();

    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_user_images(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    request: Request,
) -> Result<Json<Vec<ImageResponse>>> {
    let user = get_authenticated_user(&request)?;
    let user_id = user.user_id;

    let limit = params.get("limit")
        .and_then(|s| s.parse().ok())
        .unwrap_or(20)
        .min(100); // Max 100 images per request

    let offset = params.get("offset")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let images = state.database.get_user_images(user_id, limit, offset).await?;

    let response: Vec<ImageResponse> = images.into_iter().map(|image| {
        ImageResponse {
            id: image.id,
            filename: image.filename,
            mime_type: image.mime_type,
            file_size: image.file_size,
            width: image.width,
            height: image.height,
            url: format!("/api/v1/images/{}", image.id),
            created_at: image.created_at,
        }
    }).collect();

    Ok(Json(response))
}
