# Pixiv Image Proxy

A high-performance reverse proxy server for Pixiv images written in Rust, featuring SSL support, S3-compatible object storage caching, and Redis-based metadata caching.

## Features

- **HTTPS Only**: Secure SSL/TLS support with no HTTP fallback
- **S3 Object Storage**: Caches images in S3-compatible storage for fast retrieval
- **Redis Caching**: Intelligent caching of 404 and server error responses to reduce upstream load
- **Async Background Processing**: Non-blocking storage operations for optimal performance
- **Modular Architecture**: Clean, maintainable code with separate modules for each component

## Architecture

The proxy follows this request flow:

1. **Request Reception**: Incoming HTTPS requests for image paths
2. **Cache Check**: Verify if the request should be rejected based on cached error states (Redis)
3. **Storage Check**: Look for the image in S3 object storage first
4. **Upstream Fetch**: If not cached, fetch from the upstream Pixiv servers
5. **Response Handling**:
   - **200 OK**: Return image, store in S3 asynchronously, clear any cached errors
   - **404 Not Found**: Cache the 404 response in Redis with 1-day TTL
   - **5xx Server Error**: Cache the error in Redis with 20-minute TTL
6. **Background Storage**: Images are stored in S3 asynchronously to avoid blocking responses

## Configuration

All configuration is done via environment variables.

### Server Settings
- `SERVER_HOST`: Server bind address (default: 0.0.0.0)
- `SERVER_PORT`: Server port (default: 443)
- `SSL_CERT_PATH`: Path to SSL certificate file
- `SSL_KEY_PATH`: Path to SSL private key file

### Upstream Settings
- `UPSTREAM_HOST`: Pixiv image server URL (default: https://i.pximg.net)
- `UPSTREAM_REFERER`: Referer header for upstream requests (default: https://www.pixiv.net/)

### S3 Storage Settings
- `S3_ENDPOINT`: S3-compatible endpoint URL
- `S3_BUCKET`: Bucket name for storing cached images
- `S3_REGION`: AWS region (default: us-east-1)
- `S3_ACCESS_KEY`: S3 access key
- `S3_SECRET_KEY`: S3 secret key

### Redis Cache Settings
- `REDIS_URL`: Redis connection URL (default: redis://localhost:6379)
- `CACHE_404_TTL`: TTL in seconds for 404 responses (default: 86400 = 1 day)
- `CACHE_ERROR_TTL`: TTL in seconds for server errors (default: 1200 = 20 minutes)

## Prerequisites

- Rust 1.70+
- Redis server
- S3-compatible object storage (AWS S3, MinIO, etc.)
- SSL certificate and private key

## Installation

1. Clone the repository:
   ```bash
   git clone https://github.com/your-username/pixiv-image-proxy.git
   cd pixiv-image-proxy
   ```

2. Copy and configure environment variables:
   ```bash
   cp .env.example .env
   # Edit .env with your configuration
   ```

3. Build the application:
   ```bash
   cargo build --release
   ```

4. Run the server:
   ```bash
   cargo run --release
   ```

## Usage

Once configured and running, the proxy will handle requests at:
```
https://your-domain.com/path/to/image.jpg
```

The server will:
- Return cached images from S3 if available
- Fetch from upstream and cache on first request
- Serve subsequent requests directly from S3
- Handle error responses intelligently with TTL-based caching

## Logging

The application uses structured logging with the `tracing` crate. Set the `RUST_LOG` environment variable to control log levels:

```bash
RUST_LOG=pixiv_image_proxy=info,tower_http=info
```

Available log levels: `error`, `warn`, `info`, `debug`, `trace`

## Performance

- **Async Architecture**: Built on Tokio for high concurrency
- **Non-blocking Storage**: Images are stored in S3 asynchronously
- **Intelligent Caching**: Reduces upstream load by caching error responses
- **Memory Efficient**: Streaming responses without buffering large images

## License

Apache License - see [LICENSE](LICENSE) file for details.