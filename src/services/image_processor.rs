use image::{ImageFormat, DynamicImage, ImageError};
use std::io::Cursor;
use crate::models::ImageTransformParams;
use crate::error::{AppError, Result};

#[cfg(feature = "libvips")]
use libvips::{VipsApp, VipsImage};

pub struct ImageProcessor;

pub struct ImageInfo {
    pub width: u32,
    pub height: u32,
    pub format: ImageFormat,
    pub has_transparency: bool,
    pub color_space: String,
}

pub struct ProcessingOptions {
    pub optimize: bool,
    pub strip_metadata: bool,
    pub progressive: bool,
    pub quality: u8,
}

impl Default for ProcessingOptions {
    fn default() -> Self {
        Self {
            optimize: true,
            strip_metadata: true,
            progressive: true,
            quality: 85,
        }
    }
}

impl ImageProcessor {
    pub fn init() -> Result<()> {
        #[cfg(feature = "libvips")]
        {
            VipsApp::new("image-hosting-server", false)
                .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to initialize libvips: {}", e)))?;
        }
        Ok(())
    }

    pub fn get_image_info(data: &[u8]) -> Result<ImageInfo> {
        #[cfg(feature = "libvips")]
        {
            Self::get_image_info_vips(data)
        }
        #[cfg(not(feature = "libvips"))]
        {
            Self::get_image_info_fallback(data)
        }
    }

    #[cfg(feature = "libvips")]
    fn get_image_info_vips(data: &[u8]) -> Result<ImageInfo> {
        let img = VipsImage::new_from_buffer(data, "")
            .map_err(|e| AppError::Internal(anyhow::anyhow!("libvips error: {}", e)))?;

        let width = img.get_width() as u32;
        let height = img.get_height() as u32;
        let bands = img.get_bands();
        let has_transparency = bands == 4 || bands == 2; // RGBA or GA
        
        let format = Self::detect_format_from_data(data)?;
        let color_space = img.get_interpretation().to_string();

        Ok(ImageInfo {
            width,
            height,
            format,
            has_transparency,
            color_space,
        })
    }

    #[cfg(not(feature = "libvips"))]
    fn get_image_info_fallback(data: &[u8]) -> Result<ImageInfo> {
        let img = image::load_from_memory(data)
            .map_err(|_| AppError::InvalidFileFormat)?;
        
        let format = image::guess_format(data)
            .map_err(|_| AppError::InvalidFileFormat)?;

        let has_transparency = match img.color() {
            image::ColorType::Rgba8 | image::ColorType::Rgba16 | 
            image::ColorType::La8 | image::ColorType::La16 => true,
            _ => false,
        };

        Ok(ImageInfo {
            width: img.width(),
            height: img.height(),
            format,
            has_transparency,
            color_space: format!("{:?}", img.color()),
        })
    }

    pub fn transform_image(data: &[u8], params: &ImageTransformParams) -> Result<Vec<u8>> {
        #[cfg(feature = "libvips")]
        {
            Self::transform_image_vips(data, params)
        }
        #[cfg(not(feature = "libvips"))]
        {
            Self::transform_image_fallback(data, params)
        }
    }

    #[cfg(feature = "libvips")]
    fn transform_image_vips(data: &[u8], params: &ImageTransformParams) -> Result<Vec<u8>> {
        let mut img = VipsImage::new_from_buffer(data, "")
            .map_err(|e| AppError::Internal(anyhow::anyhow!("libvips error: {}", e)))?;

        // Auto-rotate based on EXIF orientation
        img = img.autorot()
            .map_err(|e| AppError::Internal(anyhow::anyhow!("libvips autorot error: {}", e)))?;

        // Resize if dimensions are specified
        if let (Some(width), Some(height)) = (params.width, params.height) {
            let scale_x = width as f64 / img.get_width() as f64;
            let scale_y = height as f64 / img.get_height() as f64;
            let scale = scale_x.min(scale_y);
            
            img = img.resize(scale)
                .map_err(|e| AppError::Internal(anyhow::anyhow!("libvips resize error: {}", e)))?;
        } else if let Some(width) = params.width {
            let scale = width as f64 / img.get_width() as f64;
            img = img.resize(scale)
                .map_err(|e| AppError::Internal(anyhow::anyhow!("libvips resize error: {}", e)))?;
        } else if let Some(height) = params.height {
            let scale = height as f64 / img.get_height() as f64;
            img = img.resize(scale)
                .map_err(|e| AppError::Internal(anyhow::anyhow!("libvips resize error: {}", e)))?;
        }

        // Determine output format and options
        let (format_str, options) = match params.format.as_deref() {
            Some("webp") => {
                let quality = params.quality.unwrap_or(85);
                ("webp", format!("[Q={}]", quality))
            }
            Some("png") => ("png", "[compression=6]".to_string()),
            Some("jpeg") | Some("jpg") => {
                let quality = params.quality.unwrap_or(85);
                ("jpeg", format!("[Q={},optimize_coding=true,strip=true]", quality))
            }
            _ => {
                let quality = params.quality.unwrap_or(85);
                ("jpeg", format!("[Q={},optimize_coding=true,strip=true]", quality))
            }
        };

        // Convert to buffer
        let buffer = img.write_to_buffer(&format!(".{}{}", format_str, options))
            .map_err(|e| AppError::Internal(anyhow::anyhow!("libvips write error: {}", e)))?;

        Ok(buffer)
    }

    #[cfg(not(feature = "libvips"))]
    fn transform_image_fallback(data: &[u8], params: &ImageTransformParams) -> Result<Vec<u8>> {
        let mut img = image::load_from_memory(data)
            .map_err(|_| AppError::InvalidFileFormat)?;

        // Resize if dimensions are specified
        if let (Some(width), Some(height)) = (params.width, params.height) {
            img = img.resize(width, height, image::imageops::FilterType::Lanczos3);
        } else if let Some(width) = params.width {
            let height = (img.height() as f32 * width as f32 / img.width() as f32) as u32;
            img = img.resize(width, height, image::imageops::FilterType::Lanczos3);
        } else if let Some(height) = params.height {
            let width = (img.width() as f32 * height as f32 / img.height() as f32) as u32;
            img = img.resize(width, height, image::imageops::FilterType::Lanczos3);
        }

        // Determine output format
        let format = match params.format.as_deref() {
            Some("webp") => ImageFormat::WebP,
            Some("png") => ImageFormat::Png,
            Some("jpeg") | Some("jpg") => ImageFormat::Jpeg,
            _ => ImageFormat::Jpeg,
        };

        // Encode image
        let mut output = Vec::new();
        let mut cursor = Cursor::new(&mut output);
        
        match format {
            ImageFormat::Jpeg => {
                let quality = params.quality.unwrap_or(85);
                let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut cursor, quality);
                img.write_with_encoder(encoder)
                    .map_err(|e| AppError::ImageProcessing(e))?;
            }
            _ => {
                img.write_to(&mut cursor, format)
                    .map_err(|e| AppError::ImageProcessing(e))?;
            }
        }

        Ok(output)
    }

    pub fn optimize_image(data: &[u8], options: &ProcessingOptions) -> Result<Vec<u8>> {
        #[cfg(feature = "libvips")]
        {
            Self::optimize_image_vips(data, options)
        }
        #[cfg(not(feature = "libvips"))]
        {
            Self::optimize_image_fallback(data, options)
        }
    }

    #[cfg(feature = "libvips")]
    fn optimize_image_vips(data: &[u8], options: &ProcessingOptions) -> Result<Vec<u8>> {
        let mut img = VipsImage::new_from_buffer(data, "")
            .map_err(|e| AppError::Internal(anyhow::anyhow!("libvips error: {}", e)))?;

        // Auto-rotate and strip metadata if requested
        if options.strip_metadata {
            img = img.autorot()
                .map_err(|e| AppError::Internal(anyhow::anyhow!("libvips autorot error: {}", e)))?;
        }

        // Detect original format
        let format = Self::detect_format_from_data(data)?;
        
        let (format_str, format_options) = match format {
            ImageFormat::Jpeg => {
                let mut opts = format!("[Q={}]", options.quality);
                if options.optimize {
                    opts = format!("[Q={},optimize_coding=true]", options.quality);
                }
                if options.strip_metadata {
                    opts = format!("[Q={},optimize_coding=true,strip=true]", options.quality);
                }
                if options.progressive {
                    opts = format!("[Q={},optimize_coding=true,strip=true,interlace=true]", options.quality);
                }
                ("jpeg", opts)
            }
            ImageFormat::Png => {
                let compression = if options.optimize { 9 } else { 6 };
                ("png", format!("[compression={}]", compression))
            }
            ImageFormat::WebP => {
                ("webp", format!("[Q={}]", options.quality))
            }
            _ => ("jpeg", format!("[Q={},optimize_coding=true,strip=true]", options.quality))
        };

        let buffer = img.write_to_buffer(&format!(".{}{}", format_str, format_options))
            .map_err(|e| AppError::Internal(anyhow::anyhow!("libvips write error: {}", e)))?;

        Ok(buffer)
    }

    #[cfg(not(feature = "libvips"))]
    fn optimize_image_fallback(data: &[u8], options: &ProcessingOptions) -> Result<Vec<u8>> {
        let img = image::load_from_memory(data)
            .map_err(|_| AppError::InvalidFileFormat)?;

        let format = image::guess_format(data)
            .map_err(|_| AppError::InvalidFileFormat)?;

        let mut output = Vec::new();
        let mut cursor = Cursor::new(&mut output);

        match format {
            ImageFormat::Jpeg => {
                let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut cursor, options.quality);
                img.write_with_encoder(encoder)
                    .map_err(|e| AppError::ImageProcessing(e))?;
            }
            ImageFormat::Png => {
                let encoder = image::codecs::png::PngEncoder::new(&mut cursor);
                img.write_with_encoder(encoder)
                    .map_err(|e| AppError::ImageProcessing(e))?;
            }
            _ => {
                img.write_to(&mut cursor, format)
                    .map_err(|e| AppError::ImageProcessing(e))?;
            }
        }

        Ok(output)
    }

    pub fn validate_image(data: &[u8], max_dimension: u32, max_file_size: usize) -> Result<()> {
        if data.len() > max_file_size {
            return Err(AppError::FileTooLarge);
        }

        let info = Self::get_image_info(data)?;
        
        if info.width > max_dimension || info.height > max_dimension {
            return Err(AppError::ImageTooLarge);
        }

        // Additional validation for suspicious files
        if info.width == 0 || info.height == 0 {
            return Err(AppError::InvalidFileFormat);
        }

        // Check for reasonable aspect ratio (prevent extremely thin images)
        let aspect_ratio = info.width as f32 / info.height as f32;
        if aspect_ratio > 100.0 || aspect_ratio < 0.01 {
            return Err(AppError::InvalidFileFormat);
        }

        Ok(())
    }

    pub fn generate_thumbnail(data: &[u8], size: u32) -> Result<Vec<u8>> {
        let params = ImageTransformParams {
            width: Some(size),
            height: Some(size),
            quality: Some(85),
            format: Some("jpeg".to_string()),
        };

        Self::transform_image(data, &params)
    }

    pub fn detect_format_from_data(data: &[u8]) -> Result<ImageFormat> {
        image::guess_format(data).map_err(|_| AppError::InvalidFileFormat)
    }

    pub fn is_animated(data: &[u8]) -> Result<bool> {
        // Simple check for animated GIF
        if data.len() > 6 && &data[0..6] == b"GIF89a" {
            // Look for multiple image descriptors (simplified check)
            let mut count = 0;
            let mut pos = 6;
            
            while pos < data.len() - 1 {
                if data[pos] == 0x21 && data[pos + 1] == 0xF9 {
                    count += 1;
                    if count > 1 {
                        return Ok(true);
                    }
                }
                pos += 1;
            }
        }
        
        Ok(false)
    }
}

// Worker pool for concurrent image processing
use tokio::sync::Semaphore;
use std::sync::Arc;

pub struct ImageProcessingPool {
    semaphore: Arc<Semaphore>,
}

impl ImageProcessingPool {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
        }
    }

    pub async fn process<F, T>(&self, task: F) -> Result<T>
    where
        F: FnOnce() -> Result<T> + Send + 'static,
        T: Send + 'static,
    {
        let _permit = self.semaphore.acquire().await
            .map_err(|_| AppError::Internal(anyhow::anyhow!("Failed to acquire processing permit")))?;

        let result = tokio::task::spawn_blocking(task).await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Processing task failed: {}", e)))?;

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ImageTransformParams;

    // Test image data (1x1 pixel PNG)
    const TEST_PNG_DATA: &[u8] = &[
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D,
        0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
        0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53, 0xDE, 0x00, 0x00, 0x00,
        0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8, 0x00, 0x00, 0x00,
        0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82
    ];

    #[test]
    fn test_detect_format_from_data() {
        let result = ImageProcessor::detect_format_from_data(TEST_PNG_DATA);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ImageFormat::Png);
    }

    #[test]
    fn test_get_image_info() {
        let result = ImageProcessor::get_image_info(TEST_PNG_DATA);
        assert!(result.is_ok());
        
        let info = result.unwrap();
        assert_eq!(info.width, 1);
        assert_eq!(info.height, 1);
        assert_eq!(info.format, ImageFormat::Png);
    }

    #[test]
    fn test_validate_image_valid() {
        let result = ImageProcessor::validate_image(TEST_PNG_DATA, 1000, 1024 * 1024);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_image_too_large_file() {
        let result = ImageProcessor::validate_image(TEST_PNG_DATA, 1000, 10);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::FileTooLarge));
    }

    #[test]
    fn test_validate_image_dimensions_too_large() {
        // This test would need a larger test image to properly test dimension limits
        let result = ImageProcessor::validate_image(TEST_PNG_DATA, 0, 1024 * 1024);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::ImageTooLarge));
    }

    #[test]
    fn test_transform_image_resize() {
        let params = ImageTransformParams {
            width: Some(100),
            height: Some(100),
            quality: Some(85),
            format: Some("jpeg".to_string()),
        };

        let result = ImageProcessor::transform_image(TEST_PNG_DATA, &params);
        assert!(result.is_ok());
        
        let output = result.unwrap();
        assert!(!output.is_empty());
    }

    #[test]
    fn test_generate_thumbnail() {
        let result = ImageProcessor::generate_thumbnail(TEST_PNG_DATA, 64);
        assert!(result.is_ok());
        
        let thumbnail = result.unwrap();
        assert!(!thumbnail.is_empty());
    }

    #[test]
    fn test_optimize_image() {
        let options = ProcessingOptions::default();
        let result = ImageProcessor::optimize_image(TEST_PNG_DATA, &options);
        assert!(result.is_ok());
        
        let optimized = result.unwrap();
        assert!(!optimized.is_empty());
    }

    #[test]
    fn test_is_animated_gif() {
        // Test with non-animated data
        let result = ImageProcessor::is_animated(TEST_PNG_DATA);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[tokio::test]
    async fn test_image_processing_pool() {
        let pool = ImageProcessingPool::new(2);
        
        let result = pool.process(|| {
            Ok("test result".to_string())
        }).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test result");
    }

    #[test]
    fn test_processing_options_default() {
        let options = ProcessingOptions::default();
        assert!(options.optimize);
        assert!(options.strip_metadata);
        assert!(options.progressive);
        assert_eq!(options.quality, 85);
    }
}
