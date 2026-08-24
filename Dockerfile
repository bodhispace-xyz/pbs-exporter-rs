# Multi-stage Docker build for PBS Exporter
# Stage 1: Build the binary
FROM rust:alpine AS builder

WORKDIR /app

# Install build dependencies
RUN apk add --no-cache \
    musl-dev \
    pkgconfig \
    openssl-dev \
    openssl-libs-static

# Copy manifests first for better layer caching
COPY Cargo.toml Cargo.lock ./

# Copy actual source code
COPY src ./src

# Build release binary with optimizations
RUN cargo build --release && \
    strip /app/target/release/pbs-exporter

# Stage 2: Runtime image (Alpine for minimal size)
FROM alpine:3.21

# Install only runtime dependencies
RUN apk add --no-cache \
    ca-certificates \
    libgcc \
    wget

# Create non-root user
RUN adduser -D -u 1000 exporter

# Copy binary from builder
COPY --from=builder /app/target/release/pbs-exporter /usr/local/bin/pbs-exporter

# Switch to non-root user
USER exporter

# Expose metrics port
EXPOSE 9101

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget --quiet --tries=1 --spider http://localhost:9101/health || exit 1

# Run the exporter
ENTRYPOINT ["/usr/local/bin/pbs-exporter"]
