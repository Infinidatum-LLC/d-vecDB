# d-vecDB Server - Binary Builds

This directory contains pre-built binaries for d-vecDB server across multiple platforms.

## Available Builds

### ✅ Built Locally (macOS)
- **macOS ARM64** (Apple Silicon) - 4.0 MB
- **macOS x64** (Intel) - 4.6 MB

### 🏗️ Build via CI/CD
For Linux and Windows builds, use the provided GitHub Actions workflow.

## Platform Support

| Platform | Architecture | Status | Binary Name |
|----------|-------------|--------|-------------|
| macOS | ARM64 (Apple Silicon) | ✅ Built | `vectordb-server-macos-arm64` |
| macOS | x64 (Intel) | ✅ Built | `vectordb-server-macos-x64` |
| Linux | x64 (glibc) | 🏗️ Via CI | `vectordb-server-linux-x64` |
| Linux | x64 (musl) | 🏗️ Via CI | `vectordb-server-linux-musl-x64` |
| Windows | x64 | 🏗️ Via CI | `vectordb-server-windows-x64.exe` |

## Quick Start

### macOS ARM64 (Apple Silicon)
```bash
chmod +x vectordb-server-macos-arm64
./vectordb-server-macos-arm64
```

### macOS x64 (Intel)
```bash
chmod +x vectordb-server-macos-x64
./vectordb-server-macos-x64
```

## Building All Platforms

### Option 1: GitHub Actions (Recommended)
1. Push the `.github/workflows/build-binaries.yml` to your repository
2. Create a tag: `git tag v0.1.7 && git push origin v0.1.7`
3. GitHub Actions will automatically build for all platforms
4. Download binaries from the GitHub Release

### Option 2: Local Docker Build (Linux)
```bash
# Build Linux binary using Docker
docker run --rm -v "$PWD/../:/project" -w /project rust:latest bash -c "
    cd server && \
    cargo build --release --bin vectordb-server && \
    cp target/release/vectordb-server /project/d-vecdb-server-builds/vectordb-server-linux-x64
"

# Build Linux musl binary (static, no libc dependency)
docker run --rm -v "$PWD/../:/project" -w /project rust:alpine sh -c "
    apk add musl-dev && \
    cd server && \
    cargo build --release --bin vectordb-server --target x86_64-unknown-linux-musl && \
    cp target/x86_64-unknown-linux-musl/release/vectordb-server /project/d-vecdb-server-builds/vectordb-server-linux-musl-x64
"
```

### Option 3: Cross-Compilation Setup
For manual cross-compilation from macOS:

**Linux:**
```bash
# Install cross-compilation tools
brew install messense/macos-cross-toolchains/x86_64-unknown-linux-gnu

# Build
cargo build --release --bin vectordb-server --target x86_64-unknown-linux-gnu
```

**Windows:**
```bash
# Install mingw-w64
brew install mingw-w64

# Build
cargo build --release --bin vectordb-server --target x86_64-pc-windows-gnu
```

## Binary Size Optimization

The release binaries are built with:
- Full optimization (`opt-level = 3`)
- Link-Time Optimization (`lto = "fat"`)
- Single codegen unit
- Stripped symbols (where applicable)

## Distribution

### For PyPI Package (d-vecdb-server)
Copy binaries to the Python package:
```bash
cp vectordb-server-* ../d-vecdb-server-python/d_vecdb_server/binaries/
```

### For GitHub Releases
Use the GitHub Actions workflow to automatically create releases with all platform binaries.

## Testing Binaries

```bash
# Check binary info
file vectordb-server-macos-arm64
otool -L vectordb-server-macos-arm64  # macOS dependencies

# Run health check
./vectordb-server-macos-arm64 --version
./vectordb-server-macos-arm64 --help
```

## Performance

All binaries are built with release optimizations:
- **Distance Calculations**: 35M+ ops/second
- **Vector Insertion**: 7K+ vectors/second
- **Vector Search**: 13K+ queries/second
- **Latency**: Sub-microsecond for vector operations

## Features

✅ WAL corruption protection with CRC32 checksumming
✅ GPU acceleration (10-50x speedup with CUDA/Metal)
✅ SIMD optimization (AVX2/SSE2, 2-3x speedup)
✅ Lock-free HNSW index
✅ Production-ready with full crash recovery

## Support

- GitHub: https://github.com/rdmurugan/d-vecDB
- Issues: https://github.com/rdmurugan/d-vecDB/issues
- PyPI: https://pypi.org/project/d-vecdb-server/
