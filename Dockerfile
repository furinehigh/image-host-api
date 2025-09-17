# syntax=docker/dockerfile:1

########################################
# Builder stage
########################################
FROM rust:1.71 as builder

# Install system dependencies that might be needed (libvips, etc.)
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        libvips-dev \
        ca-certificates \
        pkg-config \
        curl \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./

# Copy the actual crate
COPY image-host-api ./image-host-api

WORKDIR /app/image-host-api

RUN cargo fetch
RUN cargo build --release --bin image-host-api

# Copy source
COPY . .

# Build final binary
RUN cargo build --release --bin image-host-api

########################################
# Runtime stage
########################################
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        libvips-tools \
        ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user (optional but more secure)
RUN useradd --no-create-home --shell /usr/sbin/nologin appuser

WORKDIR /app

# Copy binary
COPY --from=builder /app/target/release/image-host-api /usr/local/bin/image-host-api

# Permissions
RUN chown appuser:appuser /usr/local/bin/image-host-api

# Drop privileges
USER appuser

# Expose the port
ENV RUST_LOG=info
ENV APP_ADDR=0.0.0.0:8080
EXPOSE 8080

# Start the application
CMD ["/usr/local/bin/image-host-api"]
