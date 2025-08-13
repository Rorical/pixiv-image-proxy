use axum::{
    extract::{Path, State},
    http::{StatusCode, header},
    response::Response,
    body::Body,
};
use bytes::Bytes;
use reqwest::Client as HttpClient;
use anyhow::Result;
use tracing::{info, error, warn};
use tokio::spawn;

use crate::{
    config::{Config, UpstreamConfig},
    storage::S3Storage,
    cache::KVStore,
};

#[derive(Clone)]
pub struct ProxyState {
    pub config: Config,
    pub storage: S3Storage,
    pub cache: KVStore,
    pub http_client: HttpClient,
}

pub async fn proxy_handler(
    Path(path): Path<String>,
    State(state): State<ProxyState>,
) -> Result<Response<Body>, (StatusCode, String)> {
    let full_path = format!("/{}", path);
    info!("Handling request for path: {}", full_path);

    // Check if we should reject this request due to cached errors
    match state.cache.should_reject(&full_path).await {
        Ok(true) => {
            return Err((StatusCode::NOT_FOUND, "Cached as unavailable".to_string()));
        },
        Ok(false) => {},
        Err(e) => {
            error!("Error checking cache: {}", e);
            // Continue processing if cache check fails
        }
    }

    // Check if file exists in S3 storage first
    match state.storage.head_object(&full_path).await {
        Ok(true) => {
            // File exists, now fetch it
            match state.storage.get_object(&full_path).await {
                Ok(Some(data)) => {
                    info!("Serving {} from S3 storage ({} bytes)", full_path, data.len());
                    return Ok(create_image_response(data, &full_path));
                },
                Ok(None) => {
                    // This shouldn't happen since head_object returned true
                    warn!("Head object succeeded but get object returned None for {}", full_path);
                },
                Err(e) => {
                    error!("Error fetching {} from S3 after successful head: {}", full_path, e);
                }
            }
        },
        Ok(false) => {
            info!("File {} not found in S3, checking upstream", full_path);
        },
        Err(e) => {
            error!("Error checking S3 storage: {}", e);
            // Continue to upstream if S3 fails
        }
    }

    // Fetch from upstream
    match fetch_from_upstream(&state.http_client, &state.config.upstream, &full_path).await {
        Ok((status, data, content_type)) => {
            match status.as_u16() {
                200 => {
                    info!("Successfully fetched {} from upstream ({} bytes)", full_path, data.len());
                    
                    // Store in S3 asynchronously
                    let storage_clone = state.storage.clone();
                    let path_clone = full_path.clone();
                    let data_clone = data.clone();
                    let content_type_clone = content_type.clone();
                    
                    spawn(async move {
                        if let Err(e) = storage_clone.put_object(&path_clone, data_clone, content_type_clone.as_deref()).await {
                            error!("Failed to store {} in S3: {}", path_clone, e);
                        }
                    });

                    // Remove any cached error status
                    if let Err(e) = state.cache.remove_cache(&full_path).await {
                        warn!("Failed to remove cache for {}: {}", full_path, e);
                    }

                    Ok(create_image_response(data, &full_path))
                },
                404 => {
                    info!("Upstream returned 404 for {}", full_path);
                    
                    // Cache 404 response
                    if let Err(e) = state.cache.cache_not_found(&full_path).await {
                        error!("Failed to cache 404 for {}: {}", full_path, e);
                    }
                    
                    Err((StatusCode::NOT_FOUND, "Image not found".to_string()))
                },
                status_code if status_code >= 500 => {
                    error!("Upstream returned server error {} for {}", status_code, full_path);
                    
                    // Cache server error
                    if let Err(e) = state.cache.cache_server_error(&full_path).await {
                        error!("Failed to cache server error for {}: {}", full_path, e);
                    }
                    
                    Err((StatusCode::BAD_GATEWAY, "Upstream server error".to_string()))
                },
                _ => {
                    warn!("Upstream returned status {} for {}", status.as_u16(), full_path);
                    Err((StatusCode::BAD_GATEWAY, format!("Upstream error: {}", status.as_u16())))
                }
            }
        },
        Err(e) => {
            error!("Failed to fetch {} from upstream: {}", full_path, e);
            
            // Cache as server error
            if let Err(cache_err) = state.cache.cache_server_error(&full_path).await {
                error!("Failed to cache server error for {}: {}", full_path, cache_err);
            }
            
            Err((StatusCode::BAD_GATEWAY, "Failed to fetch from upstream".to_string()))
        }
    }
}

async fn fetch_from_upstream(
    client: &HttpClient,
    config: &UpstreamConfig,
    path: &str,
) -> Result<(reqwest::StatusCode, Bytes, Option<String>)> {
    let url = format!("{}{}", config.host, path);
    
    let response = client
        .get(&url)
        .header("Referer", &config.referer)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .send()
        .await?;

    let status = response.status();
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|ct| ct.to_str().ok())
        .map(|s| s.to_string());
    
    let data = response.bytes().await?;
    
    Ok((status, data, content_type))
}

fn create_image_response(data: Bytes, path: &str) -> Response<Body> {
    let mut response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CACHE_CONTROL, "public, max-age=604800") // 7 days
        .header("X-Cache-Status", "HIT");

    // Set content type based on file extension
    if let Some(ext) = path.split('.').last() {
        let content_type = match ext.to_lowercase().as_str() {
            "jpg" | "jpeg" => "image/jpeg",
            "png" => "image/png",
            "gif" => "image/gif",
            "webp" => "image/webp",
            "svg" => "image/svg+xml",
            _ => "application/octet-stream",
        };
        response = response.header(header::CONTENT_TYPE, content_type);
    }

    response
        .header(header::CONTENT_LENGTH, data.len())
        .body(Body::from(data))
        .unwrap_or_else(|_| {
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("Failed to create response"))
                .unwrap()
        })
}