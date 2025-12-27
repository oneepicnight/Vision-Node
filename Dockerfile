# Vision Node - Production Multi-Stage Dockerfile
# Optimized for size, security, and performance

# ============================================
# Stage 1: Builder - Compile the application
# ============================================
FROM rust:1.75-slim as builder

WORKDIR /build

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Copy dependency manifests first (cache optimization)
COPY Cargo.toml Cargo.lock ./
COPY build.rs ./

# Create dummy source to cache dependencies
RUN mkdir -p src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release --bin vision-node && \
    rm -rf src

# Copy actual source code
COPY src ./src
COPY config ./config

# Build the application with optimizations
RUN cargo build --release --bin vision-node && \
    strip target/release/vision-node

# ============================================
# Stage 2: Runtime - Minimal production image
# ============================================
FROM debian:bookworm-slim

# Install runtime dependencies only
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

# Create non-root user for security
RUN useradd -m -u 1000 -s /bin/bash visionnode

# Create application directories
RUN mkdir -p /app/data /app/config /app/logs /app/backups && \
    chown -R visionnode:visionnode /app

WORKDIR /app

# Copy compiled binary from builder
COPY --from=builder /build/target/release/vision-node /app/vision-node

# Copy configuration files
COPY config/*.toml /app/config/

# Copy startup scripts
COPY docker-entrypoint.sh /app/
RUN chmod +x /app/docker-entrypoint.sh /app/vision-node

# Switch to non-root user
USER visionnode

# Expose ports
EXPOSE 7070
EXPOSE 9090

# Set environment variables
ENV RUST_LOG=info \
    VISION_PORT=7070 \
    VISION_DATA_DIR=/app/data \
    VISION_LOG_DIR=/app/logs

# Volume mounts for persistence
VOLUME ["/app/data", "/app/logs", "/app/backups"]

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=60s --retries=3 \
    CMD curl -f http://localhost:7070/health/live || exit 1

# Use entrypoint for graceful shutdown
ENTRYPOINT ["/app/docker-entrypoint.sh"]
CMD ["/app/vision-node"]
