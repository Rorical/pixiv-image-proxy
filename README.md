# Pixiv Image Proxy

A high-performance reverse proxy server for Pixiv images written in Rust, featuring flexible HTTP/HTTPS support, S3-compatible object storage caching with optional encryption and compression, and Redis-based metadata caching.

## Features

- **Flexible Protocol Support**: HTTP/HTTPS with automatic detection based on SSL configuration
- **S3 Object Storage**: Caches images in S3-compatible storage for fast retrieval
- **Optional Encryption**: AES-256-GCM encryption for cached objects
- **Optional Compression**: Gzip compression to reduce storage costs and transfer times
- **Redis Caching**: Intelligent caching of 404 and server error responses to reduce upstream load
- **Async Background Processing**: Non-blocking storage operations for optimal performance
- **Modular Architecture**: Clean, maintainable code with separate modules for each component
- **Docker Support**: Ready-to-use Docker configuration for easy deployment

## Architecture

The proxy follows this request flow:

1. **Request Reception**: Incoming HTTP/HTTPS requests for image paths
2. **Cache Check**: Verify if the request should be rejected based on cached error states (Redis)
3. **Storage Check**: Look for the image in S3 object storage first
4. **Data Processing**: Decrypt and/or decompress cached data if enabled
5. **Upstream Fetch**: If not cached, fetch from the upstream Pixiv servers
6. **Response Handling**:
   - **200 OK**: Return image, compress/encrypt and store in S3 asynchronously, clear any cached errors
   - **404 Not Found**: Cache the 404 response in Redis with configurable TTL
   - **5xx Server Error**: Cache the error in Redis with configurable TTL
7. **Background Storage**: Images are processed (compressed/encrypted) and stored in S3 asynchronously

## Configuration

All configuration is done via environment variables.

### Server Settings
- `SERVER_HOST`: Server bind address (default: 0.0.0.0)
- `SERVER_PORT`: Server port (default: 8080 for HTTP, 443 for HTTPS)
- `SSL_CERT_PATH`: Path to SSL certificate file (optional - enables HTTPS when provided)
- `SSL_KEY_PATH`: Path to SSL private key file (optional - enables HTTPS when provided)

**Protocol Selection:**
- **HTTP Mode**: When SSL certificate paths are not provided (default)
- **HTTPS Mode**: When both `SSL_CERT_PATH` and `SSL_KEY_PATH` are set

### Upstream Settings
- `UPSTREAM_HOST`: Pixiv image server URL (default: https://i.pximg.net)
- `UPSTREAM_REFERER`: Referer header for upstream requests (default: https://www.pixiv.net/)

### S3 Storage Settings
- `S3_ENDPOINT`: S3-compatible endpoint URL
- `S3_BUCKET`: Bucket name for storing cached images
- `S3_REGION`: AWS region (default: us-east-1)
- `S3_ACCESS_KEY`: S3 access key
- `S3_SECRET_KEY`: S3 secret key

### S3 Encryption Settings (Optional)
- `S3_ENCRYPTION_ENABLED`: Enable encryption for cached objects (true/false, default: false)
- `S3_ENCRYPTION_ALGORITHM`: Encryption algorithm (default: AES-256-GCM)
- `S3_ENCRYPTION_KEY`: Base64-encoded 32-byte encryption key (required if encryption enabled)

### S3 Compression Settings (Optional)
- `S3_COMPRESSION_ENABLED`: Enable compression for cached objects (true/false, default: false)
- `S3_COMPRESSION_ALGORITHM`: Compression algorithm (default: gzip)
- `S3_COMPRESSION_LEVEL`: Compression level 1-9 (default: 6)

### Redis Cache Settings
- `REDIS_URL`: Redis connection URL (default: redis://localhost:6379)
- `CACHE_404_TTL`: TTL in seconds for 404 responses (default: 86400 = 1 day)
- `CACHE_ERROR_TTL`: TTL in seconds for server errors (default: 1200 = 20 minutes)

## Prerequisites

- Rust 1.70+
- Redis server
- S3-compatible object storage (AWS S3, MinIO, etc.)
- SSL certificate and private key (optional, for HTTPS mode)
- Docker (optional, for containerized deployment)

## Quick Start

### Option 1: Docker (Recommended)

#### Using GitHub Container Registry (GHCR)

1. Pull the latest image:
   ```bash
   docker pull ghcr.io/rorical/pixiv-image-proxy:latest
   ```

2. Run with Docker Compose:
   ```bash
   # Create docker-compose.yml or clone the repository
   git clone https://github.com/Rorical/pixiv-image-proxy.git
   cd pixiv-image-proxy
   
   # Configure your environment variables
   docker-compose up -d
   ```

3. Or run directly with Docker:
   ```bash
   # Basic HTTP setup
   docker run -d \
     --name pixiv-proxy \
     -p 8080:8080 \
     -e S3_ENDPOINT=http://your-s3-endpoint \
     -e S3_BUCKET=pixiv-cache \
     -e S3_ACCESS_KEY=your-access-key \
     -e S3_SECRET_KEY=your-secret-key \
     -e REDIS_URL=redis://your-redis:6379 \
     ghcr.io/rorical/pixiv-image-proxy:latest
   
   # With encryption and compression
   docker run -d \
     --name pixiv-proxy \
     -p 8080:8080 \
     -e S3_ENDPOINT=http://your-s3-endpoint \
     -e S3_BUCKET=pixiv-cache \
     -e S3_ACCESS_KEY=your-access-key \
     -e S3_SECRET_KEY=your-secret-key \
     -e REDIS_URL=redis://your-redis:6379 \
     -e S3_ENCRYPTION_ENABLED=true \
     -e S3_ENCRYPTION_KEY=your_base64_key_here \
     -e S3_COMPRESSION_ENABLED=true \
     -e S3_COMPRESSION_LEVEL=6 \
     ghcr.io/rorical/pixiv-image-proxy:latest
   
   # HTTPS mode with certificates mounted
   docker run -d \
     --name pixiv-proxy \
     -p 443:443 \
     -v /path/to/certs:/app/certs:ro \
     -e SERVER_PORT=443 \
     -e SSL_CERT_PATH=/app/certs/cert.pem \
     -e SSL_KEY_PATH=/app/certs/key.pem \
     -e S3_ENDPOINT=http://your-s3-endpoint \
     -e S3_BUCKET=pixiv-cache \
     -e S3_ACCESS_KEY=your-access-key \
     -e S3_SECRET_KEY=your-secret-key \
     -e REDIS_URL=redis://your-redis:6379 \
     ghcr.io/rorical/pixiv-image-proxy:latest
   ```

#### Building from Source

1. Clone the repository:
   ```bash
   git clone https://github.com/Rorical/pixiv-image-proxy.git
   cd pixiv-image-proxy
   ```

2. Configure environment variables in `.env` file:
   ```bash
   # Required S3 configuration
   S3_ENDPOINT=http://localhost:9000
   S3_BUCKET=pixiv-cache
   S3_ACCESS_KEY=admin
   S3_SECRET_KEY=password123
   ```

3. Start with Docker Compose (HTTP mode):
   ```bash
   docker-compose up -d
   ```
   Server will be available at `http://localhost:8080`

4. For HTTPS mode, use the override file:
   ```bash
   # Place SSL certificates in ./certs/ directory
   docker-compose -f docker-compose.yml -f docker-compose.https.yml up -d
   ```

### Option 2: Manual Installation

1. Clone and build:
   ```bash
   git clone https://github.com/Rorical/pixiv-image-proxy.git
   cd pixiv-image-proxy
   cargo build --release
   ```

2. Set required environment variables:
   ```bash
   export S3_ENDPOINT=http://localhost:9000
   export S3_BUCKET=pixiv-cache
   export S3_ACCESS_KEY=admin
   export S3_SECRET_KEY=password123
   export REDIS_URL=redis://localhost:6379
   ```

3. Run the server:
   ```bash
   cargo run --release
   ```

## Usage

Once configured and running, the proxy will handle requests at:
```
http://your-domain.com/path/to/image.jpg   # HTTP mode
https://your-domain.com/path/to/image.jpg  # HTTPS mode
```

The server will:
- Return cached images from S3 if available (with optional decryption/decompression)
- Fetch from upstream and cache on first request (with optional compression/encryption)
- Serve subsequent requests directly from S3
- Handle error responses intelligently with TTL-based caching

### Advanced Configuration Examples

#### With Encryption and Compression
```bash
# Generate encryption key
cargo run --example generate_key

# Configure environment
export S3_ENCRYPTION_ENABLED=true
export S3_ENCRYPTION_KEY=your_base64_key_here
export S3_COMPRESSION_ENABLED=true
export S3_COMPRESSION_LEVEL=9
```

#### SSL Certificate Setup
```bash
# For development (self-signed)
mkdir -p certs
openssl req -x509 -newkey rsa:4096 -keyout certs/key.pem -out certs/cert.pem -days 365 -nodes -subj "/CN=localhost"

# Configure for HTTPS
export SSL_CERT_PATH=/path/to/cert.pem
export SSL_KEY_PATH=/path/to/key.pem
export SERVER_PORT=443
```

## Logging

The application uses structured logging with the `tracing` crate. Set the `RUST_LOG` environment variable to control log levels:

```bash
RUST_LOG=pixiv_image_proxy=info,tower_http=info
```

Available log levels: `error`, `warn`, `info`, `debug`, `trace`

## Security & Encryption

### Encryption Features
- **AES-256-GCM**: Industry-standard encryption for cached data
- **Key Management**: Base64-encoded keys for easy configuration
- **Automatic Processing**: Transparent encryption/decryption during storage/retrieval

### Compression Features  
- **Gzip Compression**: Reduces storage costs and transfer times
- **Configurable Levels**: Balance between compression ratio and processing time
- **Processing Order**: Compress first, then encrypt for optimal security

### Security Best Practices
- Store encryption keys securely (environment variables, secrets management)
- Use HTTPS in production environments
- Regularly rotate encryption keys
- Monitor access patterns and logs

## Performance

- **Async Architecture**: Built on Tokio for high concurrency
- **Non-blocking Storage**: Images are stored in S3 asynchronously with optional processing
- **Intelligent Caching**: Reduces upstream load by caching error responses
- **Memory Efficient**: Streaming responses without buffering large images
- **Optional Compression**: Reduces bandwidth usage and storage costs
- **Flexible Protocol**: HTTP for development, HTTPS for production

## Environment Variables Reference

### Required Variables
- `S3_ENDPOINT` - S3 endpoint URL
- `S3_BUCKET` - S3 bucket name
- `S3_ACCESS_KEY` - S3 access key
- `S3_SECRET_KEY` - S3 secret key

### Optional Variables with Defaults
| Variable | Default | Description |
|----------|---------|-------------|
| `SERVER_HOST` | `0.0.0.0` | Server bind address |
| `SERVER_PORT` | `8080` (HTTP) / `443` (HTTPS) | Server port |
| `SSL_CERT_PATH` | - | SSL certificate path (enables HTTPS) |
| `SSL_KEY_PATH` | - | SSL private key path (enables HTTPS) |
| `UPSTREAM_HOST` | `https://i.pximg.net` | Pixiv image server URL |
| `UPSTREAM_REFERER` | `https://www.pixiv.net/` | Referer header |
| `S3_REGION` | `us-east-1` | S3 region |
| `REDIS_URL` | `redis://localhost:6379` | Redis connection URL |
| `CACHE_404_TTL` | `86400` | TTL for 404 responses (seconds) |
| `CACHE_ERROR_TTL` | `1200` | TTL for server errors (seconds) |
| `S3_ENCRYPTION_ENABLED` | `false` | Enable object encryption |
| `S3_ENCRYPTION_ALGORITHM` | `AES-256-GCM` | Encryption algorithm |
| `S3_ENCRYPTION_KEY` | - | Base64 encryption key (required if enabled) |
| `S3_COMPRESSION_ENABLED` | `false` | Enable object compression |
| `S3_COMPRESSION_ALGORITHM` | `gzip` | Compression algorithm |
| `S3_COMPRESSION_LEVEL` | `6` | Compression level (1-9) |
| `RUST_LOG` | - | Logging configuration |

## Troubleshooting

### Common Issues

1. **Server won't start with SSL error**
   - Check certificate paths are correct
   - Verify certificate and key files exist and are readable
   - For HTTP mode, remove or comment out `SSL_CERT_PATH` and `SSL_KEY_PATH`

2. **S3 connection failed**
   - Verify S3 endpoint is reachable
   - Check access credentials are correct
   - Ensure bucket exists and has proper permissions

3. **Redis connection failed**
   - Verify Redis server is running
   - Check Redis URL format: `redis://host:port`
   - Test connection with `redis-cli`

4. **Encryption key errors**
   - Generate key with: `cargo run --example generate_key`
   - Ensure key is exactly 32 bytes (256 bits) when base64 decoded
   - Check key doesn't contain whitespace or newlines

5. **Performance issues**
   - Adjust compression level (lower = faster, higher = better compression)
   - Monitor resource usage (CPU, memory, network)
   - Consider disabling encryption/compression for high-throughput scenarios

### Development Tips

```bash
# Enable debug logging
export RUST_LOG=pixiv_image_proxy=debug,tower_http=debug

# Test with minimal configuration
unset SSL_CERT_PATH SSL_KEY_PATH
export SERVER_PORT=8080
export S3_ENCRYPTION_ENABLED=false
export S3_COMPRESSION_ENABLED=false
```

## License

Apache License - see [LICENSE](LICENSE) file for details.