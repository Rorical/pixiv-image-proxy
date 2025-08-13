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