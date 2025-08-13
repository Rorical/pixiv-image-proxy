use anyhow::{Result, anyhow};
use bytes::Bytes;
use reqwest::Client as HttpClient;
use rusty_s3::{Bucket, Credentials, S3Action};
use std::time::{Duration, SystemTime};
use tracing::{info, error};

use crate::config::StorageConfig;

#[derive(Clone)]
pub struct S3Storage {
    client: HttpClient,
    bucket: Bucket,
    credentials: Credentials,
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

        Ok(Self {
            client,
            bucket,
            credentials,
        })
    }

    pub async fn get_object(&self, key: &str) -> Result<Option<Bytes>> {
        let action = self.bucket.get_object(Some(&self.credentials), key);
        let url = action.sign(Duration::from_secs(3600));

        match self.client.get(url).send().await {
            Ok(response) => {
                match response.status().as_u16() {
                    200 => {
                        let data = response.bytes().await
                            .map_err(|e| anyhow!("Failed to read response body: {}", e))?;
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

    pub async fn put_object(&self, key: &str, data: Bytes, content_type: Option<&str>) -> Result<()> {
        let action = self.bucket.put_object(Some(&self.credentials), key);
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
                    error!("Failed to store object {}: HTTP {}", key, response.status());
                    Err(anyhow!("Failed to store object: HTTP {}", response.status()))
                }
            },
            Err(e) => {
                error!("Failed to store object {}: {}", key, e);
                Err(anyhow!("Failed to store object: {}", e))
            }
        }
    }

    pub async fn head_object(&self, key: &str) -> Result<bool> {
        let action = self.bucket.head_object(Some(&self.credentials), key);
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