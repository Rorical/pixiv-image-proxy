use anyhow::{Result, anyhow};
use bytes::Bytes;
use reqwest::Client as HttpClient;
use rusty_s3::{Bucket, Credentials, S3Action};
use std::time::Duration;
use tracing::{info, error};

use crate::config::StorageConfig;
use crate::crypto::CryptoProcessor;

#[derive(Clone)]
pub struct S3Storage {
    client: HttpClient,
    bucket: Bucket,
    credentials: Credentials,
    crypto_processor: Option<CryptoProcessor>,
}

impl S3Storage {
    pub async fn new(config: &StorageConfig) -> Result<Self> {
        let client = HttpClient::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| anyhow!("Failed to create HTTP client: {}", e))?;

        let bucket = Bucket::new(
            config.endpoint.parse().map_err(|e| anyhow!("Invalid S3 endpoint: {}", e))?,
            rusty_s3::UrlStyle::Path,
            config.bucket.clone(),
            config.region.clone(),
        ).map_err(|e| anyhow!("Failed to create S3 bucket: {}", e))?;

        let credentials = Credentials::new(&config.access_key, &config.secret_key);

        // Initialize crypto processor if encryption or compression is enabled
        let crypto_processor = if config.encryption.enabled || config.compression.enabled {
            Some(CryptoProcessor::new(config.encryption.clone(), config.compression.clone())?)
        } else {
            None
        };

        let storage = Self {
            client,
            bucket,
            credentials,
            crypto_processor,
        };

        // Check if bucket exists and create if necessary
        info!("Checking S3 bucket: {}", config.bucket);
        match storage.ensure_bucket_exists().await {
            Ok(_) => info!("S3 bucket '{}' is ready", config.bucket),
            Err(e) => {
                error!("Failed to ensure S3 bucket exists: {}", e);
                error!("Please verify:");
                error!("- S3_ENDPOINT: {}", config.endpoint);
                error!("- S3_BUCKET: {}", config.bucket);
                error!("- S3_REGION: {}", config.region);
                error!("- Access credentials have bucket creation permissions");
                return Err(e);
            }
        }

        Ok(storage)
    }

    pub async fn ensure_bucket_exists(&self) -> Result<()> {
        // First, try to check if bucket exists by doing a HEAD request
        match self.check_bucket_exists().await {
            Ok(true) => {
                info!("Bucket exists and is accessible");
                Ok(())
            },
            Ok(false) => {
                info!("Bucket does not exist, attempting to create it");
                self.create_bucket().await
            },
            Err(e) => {
                error!("Error checking bucket existence: {}", e);
                info!("Attempting to create bucket anyway");
                self.create_bucket().await
            }
        }
    }

    pub async fn check_bucket_exists(&self) -> Result<bool> {
        let action = self.bucket.head_bucket(Some(&self.credentials));
        let url = action.sign(Duration::from_secs(300));

        match self.client.head(url).send().await {
            Ok(response) => {
                match response.status().as_u16() {
                    200 => Ok(true),
                    404 => Ok(false),
                    403 => Err(anyhow!("Access denied - check S3 credentials and permissions")),
                    status => Err(anyhow!("Unexpected status when checking bucket: {}", status)),
                }
            },
            Err(e) => Err(anyhow!("Failed to connect to S3 endpoint: {}", e)),
        }
    }

    pub async fn create_bucket(&self) -> Result<()> {
        let action = self.bucket.create_bucket(&self.credentials);
        let url = action.sign(Duration::from_secs(300));

        match self.client.put(url).send().await {
            Ok(response) => {
                match response.status().as_u16() {
                    200 | 201 => {
                        info!("Successfully created bucket: {}", self.bucket.name());
                        Ok(())
                    },
                    409 => {
                        info!("Bucket already exists: {}", self.bucket.name());
                        Ok(())
                    },
                    403 => Err(anyhow!("Access denied - check S3 credentials have bucket creation permissions")),
                    status => {
                        let body = response.text().await.unwrap_or_default();
                        Err(anyhow!("Failed to create bucket with status {}: {}", status, body))
                    }
                }
            },
            Err(e) => Err(anyhow!("Failed to create bucket: {}", e)),
        }
    }

    pub async fn get_object(&self, key: &str) -> Result<Option<Bytes>> {
        // Normalize the key by removing leading slash
        let normalized_key = key.strip_prefix('/').unwrap_or(key);
        
        let action = self.bucket.get_object(Some(&self.credentials), normalized_key);
        let url = action.sign(Duration::from_secs(3600));

        match self.client.get(url).send().await {
            Ok(response) => {
                match response.status().as_u16() {
                    200 => {
                        let mut data = response.bytes().await
                            .map_err(|e| anyhow!("Failed to read response body: {}", e))?;
                        
                        // Decrypt and/or decompress if crypto processor is available
                        if let Some(ref processor) = self.crypto_processor {
                            data = processor.process_for_retrieval(data).await?;
                        }
                        
                        Ok(Some(data))
                    },
                    404 => Ok(None),
                    status => {
                        error!("S3 GET request failed with status {}", status);
                        Err(anyhow!("S3 GET request failed with status {}", status))
                    }
                }
            },
            Err(e) => {
                error!("Failed to get object {}: {}", key, e);
                Err(anyhow!("Failed to get object: {}", e))
            }
        }
    }

    pub async fn put_object(&self, key: &str, mut data: Bytes, content_type: Option<&str>) -> Result<()> {
        // Normalize the key by removing leading slash
        let normalized_key = key.strip_prefix('/').unwrap_or(key);
        
        // Compress and/or encrypt if crypto processor is available
        if let Some(ref processor) = self.crypto_processor {
            data = processor.process_for_storage(data).await?;
        }
        
        let action = self.bucket.put_object(Some(&self.credentials), normalized_key);
        let url = action.sign(Duration::from_secs(3600));

        let mut request = self.client
            .put(url)
            .body(data);

        if let Some(ct) = content_type {
            request = request.header("Content-Type", ct);
        }

        match request.send().await {
            Ok(response) => {
                if response.status().is_success() {
                    info!("Successfully stored object: {}", key);
                    Ok(())
                } else {
                    let status = response.status();
                    let body = response.text().await.unwrap_or_default();
                    error!("Failed to store object {}: HTTP {} - {}", key, status, body);
                    
                    // Provide specific error messages for common issues
                    match status.as_u16() {
                        404 => error!("Bucket '{}' not found. Make sure the bucket exists and the endpoint is correct.", self.bucket.name()),
                        403 => error!("Access denied. Check S3 credentials and bucket permissions for '{}'.", self.bucket.name()),
                        400 => error!("Bad request. Check the object key format: '{}'", key),
                        _ => {}
                    }
                    
                    Err(anyhow!("Failed to store object: HTTP {} - {}", status, body))
                }
            },
            Err(e) => {
                error!("Failed to store object {}: {}", key, e);
                Err(anyhow!("Failed to store object: {}", e))
            }
        }
    }

    pub async fn head_object(&self, key: &str) -> Result<bool> {
        // Normalize the key by removing leading slash
        let normalized_key = key.strip_prefix('/').unwrap_or(key);
        
        let action = self.bucket.head_object(Some(&self.credentials), normalized_key);
        let url = action.sign(Duration::from_secs(3600));

        match self.client.head(url).send().await {
            Ok(response) => {
                match response.status().as_u16() {
                    200 => Ok(true),
                    404 => Ok(false),
                    status => {
                        error!("S3 HEAD request failed with status {}", status);
                        Err(anyhow!("S3 HEAD request failed with status {}", status))
                    }
                }
            },
            Err(e) => {
                error!("Failed to check object {}: {}", key, e);
                Err(anyhow!("Failed to check object: {}", e))
            }
        }
    }
}