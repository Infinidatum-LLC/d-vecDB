# d-vecDB PyPI Installation Guide

## ✅ Package Successfully Published on PyPI

The d-vecDB Python client is available on PyPI and can be installed with a single command!

---

## 📦 Package Information

- **Package Name**: `d-vecdb`
- **PyPI URL**: https://pypi.org/project/d-vecdb/
- **Available Versions**:
  - 0.2.1 (Latest)
  - 0.2.0
  - 0.1.1
  - 0.1.0

---

## 🚀 Quick Installation

### Install Latest Version

```bash
pip install d-vecdb
```

### Install Specific Version

```bash
# Install version with recovery patch (when published)
pip install d-vecdb==0.2.2

# Install latest current version
pip install d-vecdb==0.2.1

# Install specific version
pip install d-vecdb==0.1.1
```

### Install with Development Tools

```bash
pip install d-vecdb[dev]
```

### Install with All Optional Dependencies

```bash
pip install d-vecdb[dev,docs,examples]
```

---

## 📚 Usage Example

### Basic Usage

```python
from vectordb_client import VectorDBClient

# Connect to d-vecDB server
client = VectorDBClient(host="localhost", port=3030)

# Create collection
client.create_collection(
    name="my_vectors",
    dimension=1536,
    distance_metric="Cosine"
)

# Insert vectors
import numpy as np
vector = np.random.rand(1536).tolist()

client.insert(
    collection="my_vectors",
    vector=vector,
    metadata={"source": "example"}
)

# Search
results = client.search(
    collection="my_vectors",
    query_vector=vector,
    limit=10
)

print(f"Found {len(results)} similar vectors")
```

### Async Usage

```python
import asyncio
from vectordb_client import AsyncVectorDBClient

async def main():
    client = AsyncVectorDBClient(host="localhost", port=3030)

    # Create collection
    await client.create_collection(
        name="my_vectors",
        dimension=1536,
        distance_metric="Cosine"
    )

    # Insert vectors
    await client.insert(
        collection="my_vectors",
        vector=[0.1] * 1536,
        metadata={"type": "test"}
    )

    # Search
    results = await client.search(
        collection="my_vectors",
        query_vector=[0.1] * 1536,
        limit=5
    )

    print(f"Found {len(results)} results")

asyncio.run(main())
```

---

## 🔧 Server Installation

### Option 1: Using Docker (Recommended)

```bash
docker pull rdmurugan/d-vecdb:latest
docker run -p 3030:3030 -v $(pwd)/data:/data rdmurugan/d-vecdb:latest
```

### Option 2: Using Pre-Built Binaries

Download from GitHub releases:
https://github.com/rdmurugan/d-vecDB/releases

```bash
# macOS (ARM64)
wget https://github.com/rdmurugan/d-vecDB/releases/download/v0.2.0/vectordb-server-macos-arm64
chmod +x vectordb-server-macos-arm64
./vectordb-server-macos-arm64

# Linux (x64)
wget https://github.com/rdmurugan/d-vecDB/releases/download/v0.2.0/vectordb-server-linux-x64
chmod +x vectordb-server-linux-x64
./vectordb-server-linux-x64
```

### Option 3: Build from Source

```bash
git clone https://github.com/rdmurugan/d-vecDB.git
cd d-vecDB
cargo build --release --bin vectordb-server
./target/release/vectordb-server
```

---

## ✨ Features

- ✅ High-performance vector similarity search
- ✅ HNSW indexing for fast nearest neighbor search
- ✅ GPU acceleration support (10-50x speedup)
- ✅ SIMD optimization (2-3x speedup)
- ✅ WAL with corruption protection
- ✅ Recovery system with soft-delete (v0.2.0+)
- ✅ Multiple distance metrics (Cosine, Euclidean, Dot Product)
- ✅ REST and gRPC APIs
- ✅ Type-safe Python client
- ✅ Async support
- ✅ Zero external dependencies

---

## 📊 Performance Benchmarks

Production benchmarks on DigitalOcean (2 vCPU, 2GB RAM):

| Batch Size | Throughput | vs Qdrant |
|-----------|------------|-----------|
| Single (1) | 315 vec/s | **15% faster** |
| Small (10) | 1,293 vec/s | Competitive |
| Medium (100) | 2,027 vec/s | Competitive |
| Large (500) | 2,262 vec/s | Competitive |

**Total Improvement**: 6.7x from baseline

---

## 🔐 Recovery Features (v0.2.0+)

### Soft-Delete Protection

```python
# Delete collection (soft-delete, recoverable for 24 hours)
client.delete_collection("my_vectors")

# List deleted collections
deleted = client.list_deleted_collections()

# Restore collection
client.restore_collection(
    backup_path="/data/.deleted/my_vectors_20251029_074542",
    new_name="my_vectors"
)
```

### Manual Backup

```python
# Create backup before risky operations
backup_path = client.backup_collection("my_vectors")
print(f"Backup created at: {backup_path}")
```

### Import Orphaned Data

```python
# Import orphaned vectors.bin/index.bin files
client.import_orphaned_collection(
    orphaned_path="/path/to/backup/my_vectors",
    collection_name="recovered_vectors",
    dimension=1536,
    distance_metric="Cosine"
)
```

---

## 🆘 Support

- **Documentation**: https://github.com/rdmurugan/d-vecDB#readme
- **Issues**: https://github.com/rdmurugan/d-vecDB/issues
- **PyPI**: https://pypi.org/project/d-vecdb/
- **GitHub**: https://github.com/rdmurugan/d-vecDB

---

## 📝 Version History

### v0.2.1 (Latest)
- Enhanced README with production benchmarks
- Updated documentation
- Performance improvements

### v0.2.0
- Recovery system with soft-delete
- Orphaned collection import
- Pre-operation backups
- Connection-resilient embedding generation
- 24-hour recovery window

### v0.1.1
- WAL corruption protection
- GPU acceleration support
- SIMD optimization
- Type-safe Python client

### v0.1.0
- Initial PyPI release
- REST and gRPC APIs
- HNSW indexing

---

## 🎉 Success!

Your d-vecDB package is live on PyPI and ready for installation worldwide!

```bash
pip install d-vecdb
```

**Downloads**: Check https://pypi.org/project/d-vecdb/ for download stats!
