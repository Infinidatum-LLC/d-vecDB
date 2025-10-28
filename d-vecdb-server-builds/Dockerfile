# Multi-stage Docker build for d-vecDB server
# Produces optimized binaries for Linux

FROM rust:1.89-alpine AS builder-musl

# Install build dependencies
RUN apk add --no-cache \
    musl-dev \
    openssl-dev \
    openssl-libs-static \
    pkgconfig

WORKDIR /build

# Copy the workspace
COPY ../Cargo.toml ../Cargo.lock ./
COPY ../server ./server
COPY ../common ./common
COPY ../storage ./storage
COPY ../vectorstore ./vectorstore
COPY ../index ./index
COPY ../proto ./proto

# Build release binary (static, musl)
RUN cd server && \
    RUSTFLAGS="-C target-feature=+crt-static" \
    cargo build --release --bin vectordb-server --target x86_64-unknown-linux-musl

# Extract binary
RUN cp target/x86_64-unknown-linux-musl/release/vectordb-server /vectordb-server-linux-musl-x64

# ============================================
# Runtime image (minimal)
# ============================================
FROM alpine:latest

WORKDIR /app

# Copy binary from builder
COPY --from=builder-musl /vectordb-server-linux-musl-x64 /usr/local/bin/vectordb-server

# Create data directory
RUN mkdir -p /data

# Expose ports
EXPOSE 8080 9090 9091

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:8080/health || exit 1

# Run as non-root
RUN addgroup -g 1000 vecdb && \
    adduser -D -u 1000 -G vecdb vecdb && \
    chown -R vecdb:vecdb /app /data

USER vecdb

ENTRYPOINT ["/usr/local/bin/vectordb-server"]
CMD ["--host", "0.0.0.0", "--port", "8080", "--data-dir", "/data"]
