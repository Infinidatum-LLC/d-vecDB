#!/bin/bash
set -e

# d-vecDB Server - Build All Platforms Script
# This script builds binaries for all supported platforms

echo "🚀 Building d-vecDB Server for all platforms..."
echo ""

BUILD_DIR="$(pwd)"
PROJECT_ROOT="$(cd .. && pwd)"

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# ============================================
# 1. macOS Builds (if on macOS)
# ============================================
if [[ "$OSTYPE" == "darwin"* ]]; then
    echo -e "${BLUE}Building for macOS...${NC}"

    cd "$PROJECT_ROOT"

    # macOS ARM64 (Apple Silicon)
    echo "  → Building macOS ARM64..."
    cargo build --release --bin vectordb-server --target aarch64-apple-darwin
    cp target/aarch64-apple-darwin/release/vectordb-server "$BUILD_DIR/vectordb-server-macos-arm64"
    echo -e "${GREEN}  ✓ macOS ARM64 built${NC}"

    # macOS x64 (Intel)
    echo "  → Building macOS x64..."
    cargo build --release --bin vectordb-server --target x86_64-apple-darwin
    cp target/x86_64-apple-darwin/release/vectordb-server "$BUILD_DIR/vectordb-server-macos-x64"
    echo -e "${GREEN}  ✓ macOS x64 built${NC}"

    echo ""
else
    echo -e "${BLUE}Skipping macOS builds (not on macOS)${NC}"
    echo ""
fi

# ============================================
# 2. Linux Builds (using Docker)
# ============================================
echo -e "${BLUE}Building for Linux using Docker...${NC}"

# Check if Docker is available
if ! command -v docker &> /dev/null; then
    echo -e "${RED}  ✗ Docker not found. Please install Docker to build Linux binaries.${NC}"
    echo ""
else
    # Linux x64 (glibc)
    echo "  → Building Linux x64 (glibc)..."
    docker run --rm \
        -v "$PROJECT_ROOT:/project" \
        -w /project \
        rust:latest \
        bash -c "cd server && cargo build --release --bin vectordb-server"

    cp "$PROJECT_ROOT/target/release/vectordb-server" "$BUILD_DIR/vectordb-server-linux-x64"
    echo -e "${GREEN}  ✓ Linux x64 (glibc) built${NC}"

    # Linux x64 (musl - static)
    echo "  → Building Linux x64 (musl - static)..."
    docker run --rm \
        -v "$PROJECT_ROOT:/project" \
        -w /project \
        rust:alpine \
        sh -c "apk add musl-dev openssl-dev openssl-libs-static pkgconfig && \
               cd server && \
               RUSTFLAGS='-C target-feature=+crt-static' \
               cargo build --release --bin vectordb-server --target x86_64-unknown-linux-musl"

    cp "$PROJECT_ROOT/target/x86_64-unknown-linux-musl/release/vectordb-server" "$BUILD_DIR/vectordb-server-linux-musl-x64"
    echo -e "${GREEN}  ✓ Linux x64 (musl) built${NC}"
    echo ""
fi

# ============================================
# 3. Windows Build (using Docker with MinGW)
# ============================================
echo -e "${BLUE}Building for Windows using Docker...${NC}"

if ! command -v docker &> /dev/null; then
    echo -e "${RED}  ✗ Docker not found. Skipping Windows build.${NC}"
    echo ""
else
    echo "  → Building Windows x64..."
    docker run --rm \
        -v "$PROJECT_ROOT:/project" \
        -w /project \
        rust:latest \
        bash -c "apt-get update && \
                 apt-get install -y mingw-w64 && \
                 rustup target add x86_64-pc-windows-gnu && \
                 cd server && \
                 cargo build --release --bin vectordb-server --target x86_64-pc-windows-gnu"

    cp "$PROJECT_ROOT/target/x86_64-pc-windows-gnu/release/vectordb-server.exe" "$BUILD_DIR/vectordb-server-windows-x64.exe"
    echo -e "${GREEN}  ✓ Windows x64 built${NC}"
    echo ""
fi

# ============================================
# 4. Summary
# ============================================
echo -e "${GREEN}============================================${NC}"
echo -e "${GREEN}Build Complete!${NC}"
echo -e "${GREEN}============================================${NC}"
echo ""
echo "Built binaries:"
ls -lh "$BUILD_DIR"/vectordb-server-* 2>/dev/null || echo "No binaries found"
echo ""

# Test binaries
echo "Testing binaries..."
for binary in "$BUILD_DIR"/vectordb-server-*; do
    if [[ -f "$binary" && "$binary" != *.exe ]]; then
        chmod +x "$binary"
        echo "  → $(basename "$binary"): $(file "$binary" | cut -d: -f2-)"
    fi
done

echo ""
echo -e "${BLUE}Next steps:${NC}"
echo "1. Test binaries: ./vectordb-server-macos-arm64 --version"
echo "2. Copy to PyPI package: cp vectordb-server-* ../d-vecdb-server-python/d_vecdb_server/binaries/"
echo "3. Create GitHub release: git tag v0.1.7 && git push origin v0.1.7"
echo ""
