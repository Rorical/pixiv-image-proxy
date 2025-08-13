mod config;
mod storage;
mod cache;
mod proxy;

use axum::{
    routing::get,
    Router,
};
use axum_server::tls_rustls::RustlsConfig;
use reqwest::Client as HttpClient;
use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    trace::TraceLayer,
};
use tracing::{info, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use anyhow::Result;

use config::Config;
use storage::S3Storage;
use cache::KVStore;
use proxy::{ProxyState, proxy_handler};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "pixiv_image_proxy=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting Pixiv Image Proxy Server");

    // Load configuration
    let config = Config::from_env().map_err(|e| {
        error!("Failed to load configuration: {}", e);
        e
    })?;

    info!("Configuration loaded successfully");
    info!("Server will listen on {}:{}", config.server.host, config.server.port);
    info!("Upstream host: {}", config.upstream.host);
    info!("S3 endpoint: {}", config.storage.endpoint);
    info!("S3 bucket: {}", config.storage.bucket);

    // Initialize S3 storage
    let storage = S3Storage::new(&config.storage).await.map_err(|e| {
        error!("Failed to initialize S3 storage: {}", e);
        e
    })?;
    info!("S3 storage initialized successfully");

    // Initialize KV store (Redis)
    let cache = KVStore::new(&config.cache).await.map_err(|e| {
        error!("Failed to initialize KV store: {}", e);
        e
    })?;
    info!("KV store initialized successfully");

    // Initialize HTTP client for upstream requests
    let http_client = HttpClient::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("pixiv-image-proxy/1.0")
        .build()
        .map_err(|e| {
            error!("Failed to create HTTP client: {}", e);
            anyhow::anyhow!("Failed to create HTTP client: {}", e)
        })?;

    // Create proxy state
    let state = ProxyState {
        config: config.clone(),
        storage,
        cache,
        http_client,
    };

    // Build the router
    let app = Router::new()
        .route("/*path", get(proxy_handler))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive()),
        )
        .with_state(state);

    // Configure TLS
    let tls_config = RustlsConfig::from_pem_file(&config.server.cert_path, &config.server.key_path)
        .await
        .map_err(|e| {
            error!("Failed to load TLS configuration: {}", e);
            anyhow::anyhow!("Failed to load TLS configuration: {}", e)
        })?;

    info!("TLS configuration loaded successfully");

    // Start the server
    let addr = format!("{}:{}", config.server.host, config.server.port);
    info!("Server starting on https://{}", addr);

    axum_server::bind_rustls(addr.parse()?, tls_config)
        .serve(app.into_make_service())
        .await
        .map_err(|e| {
            error!("Server error: {}", e);
            anyhow::anyhow!("Server error: {}", e)
        })?;

    Ok(())
}