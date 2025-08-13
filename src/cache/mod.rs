use redis::{Client, AsyncCommands, RedisResult};
use anyhow::{Result, anyhow};
use tracing::{info, error};
use serde::{Serialize, Deserialize};

use crate::config::CacheConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CacheStatus {
    NotFound,
    ServerError,
}

#[derive(Clone)]
pub struct KVStore {
    client: Client,
    not_found_ttl: u64,
    server_error_ttl: u64,
}

impl KVStore {
    pub async fn new(config: &CacheConfig) -> Result<Self> {
        let client = Client::open(config.redis_url.clone())
            .map_err(|e| anyhow!("Failed to connect to Redis: {}", e))?;

        // Test connection
        let mut conn = client.get_async_connection().await
            .map_err(|e| anyhow!("Failed to get Redis connection: {}", e))?;
        
        let _: String = conn.ping().await
            .map_err(|e| anyhow!("Failed to ping Redis: {}", e))?;

        info!("Successfully connected to Redis");

        Ok(Self {
            client,
            not_found_ttl: config.not_found_ttl,
            server_error_ttl: config.server_error_ttl,
        })
    }

    pub async fn should_reject(&self, path: &str) -> Result<bool> {
        let mut conn = self.client.get_async_connection().await?;
        let key = format!("cache:{}", path);
        
        let result: RedisResult<String> = conn.get(&key).await;
        match result {
            Ok(value) => {
                match serde_json::from_str::<CacheStatus>(&value) {
                    Ok(CacheStatus::NotFound) => {
                        info!("Request {} rejected due to cached 404", path);
                        Ok(true)
                    },
                    Ok(CacheStatus::ServerError) => {
                        info!("Request {} rejected due to cached server error", path);
                        Ok(true)
                    },
                    Err(_) => Ok(false),
                }
            },
            Err(_) => Ok(false), // Key doesn't exist, allow request
        }
    }

    pub async fn cache_not_found(&self, path: &str) -> Result<()> {
        let mut conn = self.client.get_async_connection().await?;
        let key = format!("cache:{}", path);
        let value = serde_json::to_string(&CacheStatus::NotFound)?;
        
        let _: RedisResult<String> = conn.set_ex(&key, value, self.not_found_ttl).await;
        info!("Cached 404 for {} with TTL {}s", path, self.not_found_ttl);
        Ok(())
    }

    pub async fn cache_server_error(&self, path: &str) -> Result<()> {
        let mut conn = self.client.get_async_connection().await?;
        let key = format!("cache:{}", path);
        let value = serde_json::to_string(&CacheStatus::ServerError)?;
        
        let _: RedisResult<String> = conn.set_ex(&key, value, self.server_error_ttl).await;
        info!("Cached server error for {} with TTL {}s", path, self.server_error_ttl);
        Ok(())
    }

    pub async fn remove_cache(&self, path: &str) -> Result<()> {
        let mut conn = self.client.get_async_connection().await?;
        let key = format!("cache:{}", path);
        
        let _: RedisResult<i32> = conn.del(&key).await;
        info!("Removed cache for {}", path);
        Ok(())
    }
}