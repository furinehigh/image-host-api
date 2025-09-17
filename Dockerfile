# Multi-stage build for Rust application
FROM rust:1.75-slim as builder

# Install system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src
COPY migrations ./migrations

# Build the application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libpq5 \
    libssl3 \
    libvips-tools \
    clamav \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -r -s /bin/false appuser

# Create directories
RUN mkdir -p /app /mnt/images && \
    chown -R appuser:appuser /app /mnt/images

# Copy binary from builder stage
COPY --from=builder /app/target/release/image-hosting-server /app/

# Copy configuration
COPY config.toml /app/

# Switch to app user
USER appuser

WORKDIR /app

EXPOSE 3000

CMD ["./image-hosting-server"]
