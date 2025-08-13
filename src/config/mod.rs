use serde::Deserialize;
use std::env;
use anyhow::Result;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub upstream: UpstreamConfig,
    pub storage: StorageConfig,
    pub cache: CacheConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub cert_path: String,
    pub key_path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpstreamConfig {
    pub host: String,
    pub referer: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StorageConfig {
    pub endpoint: String,
    pub bucket: String,
    pub region: String,
    pub access_key: String,
    pub secret_key: String,
    #[serde(default)]
    pub encryption: EncryptionConfig,
    #[serde(default)]
    pub compression: CompressionConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EncryptionConfig {
    pub enabled: bool,
    #[serde(default = "default_encryption_algorithm")]
    pub algorithm: String,
    pub key: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CompressionConfig {
    pub enabled: bool,
    #[serde(default = "default_compression_algorithm")]
    pub algorithm: String,
    #[serde(default = "default_compression_level")]
    pub level: u32,
}

impl Default for EncryptionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            algorithm: "AES-256-GCM".to_string(),
            key: None,
        }
    }
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            algorithm: "gzip".to_string(),
            level: 6,
        }
    }
}

fn default_encryption_algorithm() -> String {
    "AES-256-GCM".to_string()
}

fn default_compression_algorithm() -> String {
    "gzip".to_string()
}

fn default_compression_level() -> u32 {
    6
}

#[derive(Debug, Clone, Deserialize)]
pub struct CacheConfig {
    pub redis_url: String,
    pub not_found_ttl: u64,    // TTL in seconds for 404 responses (1 day = 86400)
    pub server_error_ttl: u64, // TTL in seconds for 5xx responses (20 min = 1200)
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Config {
            server: ServerConfig {
                host: env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
                port: env::var("SERVER_PORT")
                    .unwrap_or_else(|_| "443".to_string())
                    .parse()
                    .unwrap_or(443),
                cert_path: env::var("SSL_CERT_PATH")?,
                key_path: env::var("SSL_KEY_PATH")?,
            },
            upstream: UpstreamConfig {
                host: env::var("UPSTREAM_HOST").unwrap_or_else(|_| "https://i.pximg.net".to_string()),
                referer: env::var("UPSTREAM_REFERER").unwrap_or_else(|_| "https://www.pixiv.net/".to_string()),
            },
            storage: StorageConfig {
                endpoint: env::var("S3_ENDPOINT")?,
                bucket: env::var("S3_BUCKET")?,
                region: env::var("S3_REGION").unwrap_or_else(|_| "us-east-1".to_string()),
                access_key: env::var("S3_ACCESS_KEY")?,
                secret_key: env::var("S3_SECRET_KEY")?,
                encryption: EncryptionConfig {
                    enabled: env::var("S3_ENCRYPTION_ENABLED")
                        .unwrap_or_else(|_| "false".to_string())
                        .parse()
                        .unwrap_or(false),
                    algorithm: env::var("S3_ENCRYPTION_ALGORITHM")
                        .unwrap_or_else(|_| "AES-256-GCM".to_string()),
                    key: env::var("S3_ENCRYPTION_KEY").ok(),
                },
                compression: CompressionConfig {
                    enabled: env::var("S3_COMPRESSION_ENABLED")
                        .unwrap_or_else(|_| "false".to_string())
                        .parse()
                        .unwrap_or(false),
                    algorithm: env::var("S3_COMPRESSION_ALGORITHM")
                        .unwrap_or_else(|_| "gzip".to_string()),
                    level: env::var("S3_COMPRESSION_LEVEL")
                        .unwrap_or_else(|_| "6".to_string())
                        .parse()
                        .unwrap_or(6),
                },
            },
            cache: CacheConfig {
                redis_url: env::var("REDIS_URL")?,
                not_found_ttl: env::var("CACHE_404_TTL")
                    .unwrap_or_else(|_| "86400".to_string())
                    .parse()
                    .unwrap_or(86400),
                server_error_ttl: env::var("CACHE_ERROR_TTL")
                    .unwrap_or_else(|_| "1200".to_string())
                    .parse()
                    .unwrap_or(1200),
            },
        })
    }
}