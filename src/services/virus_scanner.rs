use reqwest::Client;
use serde::{Deserialize, Serialize};
use crate::error::{AppError, Result};
use std::time::Duration;

#[derive(Debug, Serialize)]
struct ScanRequest {
    data: String, // base64 encoded file data
    filename: String,
}

#[derive(Debug, Deserialize)]
struct ScanResponse {
    clean: bool,
    threats: Vec<String>,
    scan_time: f64,
}

pub struct VirusScanner {
    client: Client,
    scan_url: String,
    enabled: bool,
}

impl VirusScanner {
    pub fn new(scan_url: Option<String>, enabled: bool) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap();

        Self {
            client,
            scan_url: scan_url.unwrap_or_default(),
            enabled,
        }
    }

    pub async fn scan_file(&self, data: &[u8], filename: &str) -> Result<bool> {
        if !self.enabled {
            return Ok(true); // Skip scanning if disabled
        }

        if self.scan_url.is_empty() {
            tracing::warn!("Virus scanning enabled but no scan URL configured");
            return Ok(true);
        }

        // Encode file data as base64
        let encoded_data = base64::encode(data);
        
        let request = ScanRequest {
            data: encoded_data,
            filename: filename.to_string(),
        };

        let response = self.client
            .post(&self.scan_url)
            .json(&request)
            .send()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Virus scan request failed: {}", e)))?;

        if !response.status().is_success() {
            tracing::error!("Virus scan service returned error: {}", response.status());
            // Fail safe - reject file if scanner is unavailable
            return Err(AppError::Internal(anyhow::anyhow!("Virus scan service unavailable")));
        }

        let scan_result: ScanResponse = response.json().await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to parse scan response: {}", e)))?;

        if !scan_result.clean {
            tracing::warn!("Virus detected in file {}: {:?}", filename, scan_result.threats);
            return Err(AppError::VirusDetected);
        }

        tracing::debug!("File {} scanned clean in {:.2}s", filename, scan_result.scan_time);
        Ok(true)
    }

    pub async fn scan_url(&self, url: &str) -> Result<bool> {
        if !self.enabled {
            return Ok(true);
        }

        // Implementation for scanning URLs (if needed)
        // This would be used for scanning images uploaded via URL
        Ok(true)
    }
}

// Add base64 dependency to Cargo.toml
