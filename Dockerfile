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
COPY rust-image-host ./rust-image-host

WORKDIR /app/rust-image-host

RUN cargo fetch
RUN cargo build --release --bin rust-image-host

# Copy source
COPY . .

# Build final binary
RUN cargo build --release --bin rust-image-host

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
COPY --from=builder /app/target/release/rust-image-host /usr/local/bin/rust-image-host

# Permissions
RUN chown appuser:appuser /usr/local/bin/rust-image-host

# Drop privileges
USER appuser

# Expose the port
ENV RUST_LOG=info
ENV APP_ADDR=0.0.0.0:8080
EXPOSE 8080

# Start the application
CMD ["/usr/local/bin/rust-image-host"]
