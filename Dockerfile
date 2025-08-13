# Multi-stage build for optimized image size
FROM rust:bookworm as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN adduser --disabled-password --gecos '' --shell /bin/bash --home /app appuser

# Set working directory
WORKDIR /app

# Copy dependency files first for better caching
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies (this layer will be cached if Cargo.toml doesn't change)
RUN cargo build --release && rm -rf src target/release/deps/pixiv*

# Copy source code
COPY src ./src

# Build the application
RUN cargo build --release

# Runtime stage - using distroless for minimal footprint
FROM gcr.io/distroless/cc-debian12

# Copy ca-certificates from builder
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/

# Copy the binary from builder stage
COPY --from=builder /app/target/release/pixiv-image-proxy /pixiv-image-proxy

# Expose port
EXPOSE 443

# Set the startup command
ENTRYPOINT ["/pixiv-image-proxy"]