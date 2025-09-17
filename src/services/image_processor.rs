use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;
use tokio::{fs, sync::mpsc, task::JoinHandle};
use uuid::Uuid;

use crate::{
    config::ImageProcessingConfig,
    database::queries::ImageQueries,
    errors::{AppError, Result},
    models::ImageVariant,
    storage::Storage,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingJob {
    pub id: Uuid,
    pub image_id: Uuid,
    pub job_type: ProcessingJobType,
    pub parameters: ProcessingParameters,
    pub status: JobStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub error_message: Option<String>,
    pub retry_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessingJobType {
    GenerateVariants,
    Resize,
    Convert,
    Optimize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingParameters {
    pub original_path: String,
    pub output_dir: String,
    pub thumbnail_sizes: Vec<u32>,
    pub generate_webp: bool,
    pub generate_avif: bool,
    pub custom_sizes: Vec<u32>,
    pub quality_webp: u8,
    pub quality_avif: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobStatus {
    Queued,
    Processing,
    Completed,
    Failed,
    Retrying,
}

pub struct ImageProcessor {
    config: ImageProcessingConfig,
    job_sender: mpsc::UnboundedSender<ProcessingJob>,
    worker_handles: Vec<JoinHandle<()>>,
}

impl ImageProcessor {
    pub fn new(config: &ImageProcessingConfig) -> Self {
        let (job_sender, job_receiver) = mpsc::unbounded_channel();
        let mut worker_handles = Vec::new();

        // Spawn worker tasks
        for worker_id in 0..config.max_workers {
            let worker_config = config.clone();
            let mut receiver = job_receiver.clone();
            
            let handle = tokio::spawn(async move {
                Self::worker_loop(worker_id, worker_config, receiver).await;
            });
            
            worker_handles.push(handle);
        }

        Self {
            config: config.clone(),
            job_sender,
            worker_handles,
        }
    }

    pub async fn queue_processing_job(
        &self,
        image_id: Uuid,
        original_path: String,
        thumbnail_sizes: Vec<u32>,
        custom_sizes: Vec<u32>,
    ) -> Result<ProcessingJob> {
        let job_id = Uuid::new_v4();
        let output_dir = format!("processed/{}", image_id);

        let job = ProcessingJob {
            id: job_id,
            image_id,
            job_type: ProcessingJobType::GenerateVariants,
            parameters: ProcessingParameters {
                original_path,
                output_dir,
                thumbnail_sizes,
                generate_webp: true,
                generate_avif: true,
                custom_sizes,
                quality_webp: self.config.quality_webp,
                quality_avif: self.config.quality_avif,
            },
            status: JobStatus::Queued,
            created_at: chrono::Utc::now(),
            started_at: None,
            completed_at: None,
            error_message: None,
            retry_count: 0,
        };

        self.job_sender.send(job.clone())
            .map_err(|_| AppError::Internal(anyhow::anyhow!("Failed to queue processing job")))?;

        tracing::info!("Queued processing job {} for image {}", job_id, image_id);
        Ok(job)
    }

    async fn worker_loop(
        worker_id: usize,
        config: ImageProcessingConfig,
        mut receiver: mpsc::UnboundedReceiver<ProcessingJob>,
    ) {
        tracing::info!("Image processing worker {} started", worker_id);

        while let Some(mut job) = receiver.recv().await {
            tracing::info!("Worker {} processing job {}", worker_id, job.id);
            
            job.status = JobStatus::Processing;
            job.started_at = Some(chrono::Utc::now());

            match Self::process_job(&config, &job).await {
                Ok(variants) => {
                    job.status = JobStatus::Completed;
                    job.completed_at = Some(chrono::Utc::now());
                    
                    // TODO: Update database with generated variants
                    tracing::info!("Job {} completed successfully with {} variants", job.id, variants.len());
                }
                Err(e) => {
                    job.status = JobStatus::Failed;
                    job.error_message = Some(e.to_string());
                    job.retry_count += 1;
                    
                    tracing::error!("Job {} failed: {}", job.id, e);
                    
                    // Retry logic
                    if job.retry_count < 3 {
                        job.status = JobStatus::Retrying;
                        tokio::time::sleep(tokio::time::Duration::from_secs(job.retry_count as u64 * 10)).await;
                        
                        // Re-queue the job
                        // In a real implementation, you'd use a proper job queue with retry logic
                        tracing::info!("Retrying job {} (attempt {})", job.id, job.retry_count + 1);
                    }
                }
            }
        }

        tracing::info!("Image processing worker {} stopped", worker_id);
    }

    async fn process_job(
        config: &ImageProcessingConfig,
        job: &ProcessingJob,
    ) -> Result<Vec<ImageVariant>> {
        let mut variants = Vec::new();
        let params = &job.parameters;

        // Create output directory
        fs::create_dir_all(&params.output_dir).await
            .map_err(|e| AppError::FileProcessing(format!("Failed to create output directory: {}", e)))?;

        // Generate thumbnails
        for &size in &params.thumbnail_sizes {
            let output_path = format!("{}/thumb_{}px.webp", params.output_dir, size);
            
            Self::resize_image(
                config,
                &params.original_path,
                &output_path,
                Some(size),
                None,
                "webp",
                params.quality_webp,
            ).await?;

            // Get dimensions of generated thumbnail
            let (width, height) = Self::get_image_dimensions(&output_path).await?;
            let size_bytes = fs::metadata(&output_path).await?.len();

            variants.push(ImageVariant {
                width,
                height,
                format: "image/webp".to_string(),
                size_bytes,
                url: format!("/v1/images/{}/thumb_{}px.webp", job.image_id, size),
            });
        }

        // Generate custom sizes
        for &size in &params.custom_sizes {
            let output_path = format!("{}/custom_{}px.webp", params.output_dir, size);
            
            Self::resize_image(
                config,
                &params.original_path,
                &output_path,
                Some(size),
                None,
                "webp",
                params.quality_webp,
            ).await?;

            let (width, height) = Self::get_image_dimensions(&output_path).await?;
            let size_bytes = fs::metadata(&output_path).await?.len();

            variants.push(ImageVariant {
                width,
                height,
                format: "image/webp".to_string(),
                size_bytes,
                url: format!("/v1/images/{}/custom_{}px.webp", job.image_id, size),
            });
        }

        // Generate WebP version
        if params.generate_webp {
            let output_path = format!("{}/optimized.webp", params.output_dir);
            
            Self::convert_format(
                config,
                &params.original_path,
                &output_path,
                "webp",
                params.quality_webp,
            ).await?;

            let (width, height) = Self::get_image_dimensions(&output_path).await?;
            let size_bytes = fs::metadata(&output_path).await?.len();

            variants.push(ImageVariant {
                width,
                height,
                format: "image/webp".to_string(),
                size_bytes,
                url: format!("/v1/images/{}/optimized.webp", job.image_id),
            });
        }

        // Generate AVIF version
        if params.generate_avif {
            let output_path = format!("{}/optimized.avif", params.output_dir);
            
            Self::convert_format(
                config,
                &params.original_path,
                &output_path,
                "avif",
                params.quality_avif,
            ).await?;

            let (width, height) = Self::get_image_dimensions(&output_path).await?;
            let size_bytes = fs::metadata(&output_path).await?.len();

            variants.push(ImageVariant {
                width,
                height,
                format: "image/avif".to_string(),
                size_bytes,
                url: format!("/v1/images/{}/optimized.avif", job.image_id),
            });
        }

        Ok(variants)
    }

    async fn resize_image(
        config: &ImageProcessingConfig,
        input_path: &str,
        output_path: &str,
        width: Option<u32>,
        height: Option<u32>,
        format: &str,
        quality: u8,
    ) -> Result<()> {
        if config.use_vips {
            Self::resize_with_vips(config, input_path, output_path, width, height, format, quality).await
        } else {
            Self::resize_with_image_crate(input_path, output_path, width, height).await
        }
    }

    async fn resize_with_vips(
        config: &ImageProcessingConfig,
        input_path: &str,
        output_path: &str,
        width: Option<u32>,
        height: Option<u32>,
        format: &str,
        quality: u8,
    ) -> Result<()> {
        let mut cmd = Command::new(&config.vips_path);

        // Build vips command
        if let Some(w) = width {
            if let Some(h) = height {
                // Resize to exact dimensions
                cmd.arg("resize")
                    .arg(input_path)
                    .arg(output_path)
                    .arg(format!("{}x{}", w, h));
            } else {
                // Resize by width, maintain aspect ratio
                cmd.arg("resize")
                    .arg(input_path)
                    .arg(output_path)
                    .arg(w.to_string());
            }
        } else {
            // Just convert format
            cmd.arg("copy")
                .arg(input_path);
            
            // Add format-specific options
            match format {
                "webp" => cmd.arg(format!("{}[Q={}]", output_path, quality)),
                "avif" => cmd.arg(format!("{}[Q={}]", output_path, quality)),
                "jpeg" | "jpg" => cmd.arg(format!("{}[Q={}]", output_path, quality)),
                _ => cmd.arg(output_path),
            };
        }

        let output = cmd.output()
            .map_err(|e| AppError::FileProcessing(format!("Failed to execute vips: {}", e)))?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::FileProcessing(format!("Vips processing failed: {}", error)));
        }

        Ok(())
    }

    async fn resize_with_image_crate(
        input_path: &str,
        output_path: &str,
        width: Option<u32>,
        height: Option<u32>,
    ) -> Result<()> {
        let img = image::open(input_path)
            .map_err(|e| AppError::FileProcessing(format!("Failed to open image: {}", e)))?;

        let processed_img = if let (Some(w), Some(h)) = (width, height) {
            img.resize_exact(w, h, image::imageops::FilterType::Lanczos3)
        } else if let Some(w) = width {
            let aspect_ratio = img.height() as f32 / img.width() as f32;
            let h = (w as f32 * aspect_ratio) as u32;
            img.resize(w, h, image::imageops::FilterType::Lanczos3)
        } else if let Some(h) = height {
            let aspect_ratio = img.width() as f32 / img.height() as f32;
            let w = (h as f32 * aspect_ratio) as u32;
            img.resize(w, h, image::imageops::FilterType::Lanczos3)
        } else {
            img
        };

        processed_img.save(output_path)
            .map_err(|e| AppError::FileProcessing(format!("Failed to save processed image: {}", e)))?;

        Ok(())
    }

    async fn convert_format(
        config: &ImageProcessingConfig,
        input_path: &str,
        output_path: &str,
        format: &str,
        quality: u8,
    ) -> Result<()> {
        Self::resize_image(config, input_path, output_path, None, None, format, quality).await
    }

    async fn get_image_dimensions(path: &str) -> Result<(u32, u32)> {
        let img = image::open(path)
            .map_err(|e| AppError::FileProcessing(format!("Failed to open image for dimensions: {}", e)))?;
        
        Ok((img.width(), img.height()))
    }

    pub async fn optimize_image(
        config: &ImageProcessingConfig,
        input_path: &str,
        output_path: &str,
    ) -> Result<()> {
        if config.use_vips {
            // Use vips for optimization
            let mut cmd = Command::new(&config.vips_path);
            cmd.arg("copy")
                .arg(input_path)
                .arg(format!("{}[strip,optimize_coding]", output_path));

            let output = cmd.output()
                .map_err(|e| AppError::FileProcessing(format!("Failed to execute vips: {}", e)))?;

            if !output.status.success() {
                let error = String::from_utf8_lossy(&output.stderr);
                return Err(AppError::FileProcessing(format!("Vips optimization failed: {}", error)));
            }
        } else {
            // Use image crate for basic optimization
            let img = image::open(input_path)
                .map_err(|e| AppError::FileProcessing(format!("Failed to open image: {}", e)))?;

            img.save(output_path)
                .map_err(|e| AppError::FileProcessing(format!("Failed to save optimized image: {}", e)))?;
        }

        Ok(())
    }

    pub async fn shutdown(self) {
        // Close the job sender to signal workers to stop
        drop(self.job_sender);

        // Wait for all workers to finish
        for handle in self.worker_handles {
            let _ = handle.await;
        }

        tracing::info!("Image processor shutdown complete");
    }
}

// Virus scanning integration
pub struct VirusScanner {
    enabled: bool,
    clamav_path: Option<String>,
}

impl VirusScanner {
    pub fn new(enabled: bool, clamav_path: Option<String>) -> Self {
        Self {
            enabled,
            clamav_path,
        }
    }

    pub async fn scan_file(&self, file_path: &str) -> Result<bool> {
        if !self.enabled {
            return Ok(true); // Skip scanning if disabled
        }

        let clamav_path = self.clamav_path.as_ref()
            .ok_or_else(|| AppError::FileProcessing("ClamAV path not configured".to_string()))?;

        let output = Command::new(clamav_path)
            .arg("--no-summary")
            .arg("--infected")
            .arg(file_path)
            .output()
            .map_err(|e| AppError::FileProcessing(format!("Failed to execute ClamAV: {}", e)))?;

        // ClamAV returns 0 for clean files, 1 for infected files
        match output.status.code() {
            Some(0) => Ok(true),  // Clean
            Some(1) => Ok(false), // Infected
            _ => {
                let error = String::from_utf8_lossy(&output.stderr);
                Err(AppError::FileProcessing(format!("ClamAV scan failed: {}", error)))
            }
        }
    }

    pub async fn scan_bytes(&self, data: &[u8]) -> Result<bool> {
        if !self.enabled {
            return Ok(true);
        }

        // Write to temporary file and scan
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join(format!("scan_{}.tmp", Uuid::new_v4()));
        
        fs::write(&temp_file, data).await
            .map_err(|e| AppError::FileProcessing(format!("Failed to write temp file for scanning: {}", e)))?;

        let result = self.scan_file(temp_file.to_str().unwrap()).await;

        // Clean up temp file
        let _ = fs::remove_file(&temp_file).await;

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_image_processor_creation() {
        let config = ImageProcessingConfig {
            use_vips: false,
            vips_path: "vips".to_string(),
            thumbnail_sizes: vec![64, 128, 256],
            quality_webp: 80,
            quality_avif: 70,
            max_workers: 2,
        };

        let processor = ImageProcessor::new(&config);
        assert_eq!(processor.worker_handles.len(), 2);
        
        processor.shutdown().await;
    }

    #[tokio::test]
    async fn test_virus_scanner() {
        let scanner = VirusScanner::new(false, None);
        let result = scanner.scan_bytes(b"test data").await.unwrap();
        assert!(result); // Should return true when disabled
    }
}
