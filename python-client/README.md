# d-vecDB Python Client

[![PyPI version](https://img.shields.io/pypi/v/d-vecdb.svg)](https://pypi.org/project/d-vecdb/)
[![Python 3.8+](https://img.shields.io/badge/python-3.8+-blue.svg)](https://www.python.org/downloads/)
[![Downloads](https://pepy.tech/badge/d-vecdb)](https://pepy.tech/project/d-vecdb)
[![License](https://img.shields.io/badge/License-Proprietary-red.svg)](https://github.com/rdmurugan/d-vecDB/blob/master/LICENSE)
[![Code style: black](https://img.shields.io/badge/code%20style-black-000000.svg)](https://github.com/psf/black)
[![Typing: typed](https://img.shields.io/badge/typing-typed-blue.svg)](https://peps.python.org/pep-0561/)

**High-performance Python client for d-vecDB vector database** - Production-ready with WAL corruption protection, GPU acceleration, and SIMD optimization.

A comprehensive Python client library for [d-vecDB](https://github.com/rdmurugan/d-vecDB), providing both synchronous and asynchronous interfaces for vector database operations.

## 🚀 **Features**

### **Multi-Protocol Support**
- **REST API** via HTTP/HTTPS with connection pooling
- **gRPC** for high-performance binary protocol communication
- **Auto-detection** with intelligent fallback

### **Synchronous & Asynchronous**
- **Sync client** for traditional blocking operations
- **Async client** for high-concurrency applications  
- **Connection pooling** and concurrent batch operations

### **Type Safety & Validation**
- **Pydantic models** for data validation
- **Type hints** throughout the codebase
- **Comprehensive error handling**

### **Developer Experience**
- **Intuitive API** with simple and advanced methods
- **NumPy integration** for seamless array handling
- **Rich documentation** and examples

## 📊 **Performance Highlights**

### **Production Performance (October 2025)**

*Benchmarked on DigitalOcean 2 vCPU, 2GB RAM*

| Batch Size | d-vecDB | Qdrant | Status |
|-----------|---------|--------|--------|
| **Single (1)** | **315 vec/s** | 275 vec/s | ✅ **15% FASTER** |
| **Small (10)** | 1,293 vec/s | 1,628 vec/s | 1.26x slower |
| **Medium (100)** | 2,027 vec/s | 3,720 vec/s | 1.84x slower |
| **Large (500)** | 2,262 vec/s | 4,244 vec/s | 1.88x slower |

**Key Achievement**: d-vecDB beats Qdrant on single insert throughput! 🏆

### **Production Features**

✅ **WAL Corruption Protection**
- CRC32 checksumming for all entries
- Magic number boundaries for corruption detection
- Graceful recovery from crashes and partial writes
- Production-grade durability

✅ **Hardware Acceleration**
- GPU acceleration with automatic CPU fallback (10-50x speedup)
- SIMD optimization (AVX2/SSE2) for 2-3x faster distance calculations
- Automatic hardware detection

---

## 📦 **Installation**

### **Quick Install from PyPI (Recommended)**

```bash
# Install the Python client
pip install d-vecdb

# Install with development dependencies
pip install d-vecdb[dev]

# Install with example dependencies
pip install d-vecdb[examples]
```

### **Install the Complete Server + Client**

For a complete zero-config setup with embedded server binaries:

```bash
# Install the server package (includes binaries for Linux, macOS, Windows)
pip install d-vecdb-server

# This automatically includes the d-vecdb client as a dependency
```

The `d-vecdb-server` package includes:
- ✅ Pre-built server binaries for all major platforms
- ✅ Zero configuration required
- ✅ Automatic platform detection
- ✅ Python client included

**Start the server:**
```bash
# Using command-line
d-vecdb-server

# Or via Python
python -m d_vecdb_server
```

### **From Source**

```bash
git clone https://github.com/rdmurugan/d-vecDB.git
cd d-vecDB/python-client
pip install -e .
```

## 🚀 **Getting Started**

### **Option 1: Using the Complete Server Package (Easiest)**

```bash
# Install everything
pip install d-vecdb-server

# Start the server (runs in foreground)
d-vecdb-server --host 0.0.0.0 --port 8080

# Or start in Python
python -c "from d_vecdb_server import start_server; start_server()"
```

### **Option 2: Install Client Only + Build Server from Source**

**Step 1: Build and Start the d-vecDB Server**

```bash
# Clone the repository and build the server
git clone https://github.com/rdmurugan/d-vecDB.git
cd d-vecDB

# Build the server (requires Rust)
cargo build --release

# Start the server
./target/release/vectordb-server --host 0.0.0.0 --port 8080
```

**Step 2: Install and Use the Python Client**

```bash
# Install the client
pip install d-vecdb
```

### **Using the Python Client**

Once you have a running d-vecDB server, connect and start working with vectors:

```python
import numpy as np
from vectordb_client import VectorDBClient

# Connect to your d-vecDB server
client = VectorDBClient(host="localhost", port=8080)

# Create a collection
client.create_collection_simple("my_collection", 128, "cosine")

# Insert some vectors
vector = np.random.random(128)
client.insert_simple("my_collection", "vector_1", vector)

# Search for similar vectors
query = np.random.random(128)
results = client.search_simple("my_collection", query, limit=5)

print(f"Found {len(results)} similar vectors")
for result in results:
    print(f"  - ID: {result.id}, Distance: {result.distance:.4f}")

client.close()
```

## 🏃 **Quick Start**

### **Synchronous Client**

```python
import numpy as np
from vectordb_client import VectorDBClient

# Connect to d-vecDB server
client = VectorDBClient(host="localhost", port=8080)

# Create a collection
client.create_collection_simple(
    name="documents", 
    dimension=128, 
    distance_metric="cosine"
)

# Insert vectors
vectors = np.random.random((100, 128))
for i, vector in enumerate(vectors):
    client.insert_simple(
        collection_name="documents",
        vector_id=f"doc_{i}",
        vector_data=vector,
        metadata={"title": f"Document {i}", "category": "example"}
    )

# Search for similar vectors
query_vector = np.random.random(128)
results = client.search_simple("documents", query_vector, limit=5)

for result in results:
    print(f"ID: {result.id}, Distance: {result.distance:.4f}")

# Clean up
client.close()
```

### **Asynchronous Client**

```python
import asyncio
import numpy as np
from vectordb_client import AsyncVectorDBClient

async def main():
    # Connect to d-vecDB server
    async with AsyncVectorDBClient(host="localhost", port=8080) as client:
        
        # Create collection
        await client.create_collection_simple(
            name="embeddings", 
            dimension=384, 
            distance_metric="cosine"
        )
        
        # Prepare batch data
        batch_data = [
            (f"item_{i}", np.random.random(384), {"category": "test"})
            for i in range(1000)
        ]
        
        # Concurrent batch insertion
        await client.batch_insert_concurrent(
            collection_name="embeddings",
            vectors_data=batch_data,
            batch_size=50,
            max_concurrent_batches=10
        )
        
        # Search
        query_vector = np.random.random(384)
        results = await client.search_simple("embeddings", query_vector, limit=10)
        
        print(f"Found {len(results)} similar vectors")

# Run the async example
asyncio.run(main())
```

## 📖 **API Reference**

### **Client Initialization**

```python
from vectordb_client import VectorDBClient, AsyncVectorDBClient

# Synchronous client
client = VectorDBClient(
    host="localhost",
    port=8080,              # REST port
    grpc_port=9090,         # gRPC port  
    protocol="rest",        # "rest", "grpc", or "auto"
    ssl=False,              # Use HTTPS/secure gRPC
    timeout=30.0,           # Request timeout
)

# Asynchronous client
async_client = AsyncVectorDBClient(
    host="localhost",
    port=8080,
    connection_pool_size=10,  # HTTP connection pool size
    protocol="rest",
    ssl=False,
    timeout=30.0,
)
```

### **Collection Management**

```python
from vectordb_client.types import CollectionConfig, DistanceMetric, IndexConfig

# Advanced collection configuration
config = CollectionConfig(
    name="my_collection",
    dimension=768,
    distance_metric=DistanceMetric.COSINE,
    index_config=IndexConfig(
        max_connections=32,
        ef_construction=400,
        ef_search=100,
        max_layer=16
    )
)

# Create collection
response = client.create_collection(config)

# List all collections
collections = client.list_collections()
print("Collections:", collections.collections)

# Get collection info and stats
collection_info = client.get_collection("my_collection")
stats = client.get_collection_stats("my_collection")
print(f"Vectors: {stats.vector_count}, Memory: {stats.memory_usage} bytes")

# Delete collection
client.delete_collection("my_collection")
```

### **Vector Operations**

```python
from vectordb_client.types import Vector
import numpy as np

# Create vectors with metadata
vectors = [
    Vector(
        id="vec_1",
        data=np.random.random(128).tolist(),
        metadata={"category": "A", "score": 0.95}
    ),
    Vector(
        id="vec_2", 
        data=np.random.random(128).tolist(),
        metadata={"category": "B", "score": 0.87}
    )
]

# Insert single vector
response = client.insert_vector("my_collection", vectors[0])

# Batch insert
response = client.insert_vectors("my_collection", vectors)
print(f"Inserted {response.inserted_count} vectors")

# Get vector by ID
vector = client.get_vector("my_collection", "vec_1")
print(f"Retrieved vector: {vector.id}")

# Update vector
vectors[0].metadata["updated"] = True
client.update_vector("my_collection", vectors[0])

# Delete vector  
client.delete_vector("my_collection", "vec_1")
```

### **Vector Search**

```python
from vectordb_client.types import SearchRequest
import numpy as np

# Simple search
query_vector = np.random.random(128)
results = client.search_simple("my_collection", query_vector, limit=10)

# Advanced search with parameters
search_request = SearchRequest(
    query_vector=query_vector.tolist(),
    limit=20,
    ef_search=150,  # Higher value = better accuracy, slower search
    filter={"category": "A"}  # Metadata filtering
)

response = client.search("my_collection", 
                        search_request.query_vector,
                        search_request.limit,
                        search_request.ef_search,
                        search_request.filter)

# Process results
for result in response.results:
    print(f"ID: {result.id}")
    print(f"Distance: {result.distance:.6f}")  
    print(f"Metadata: {result.metadata}")
    print("---")

print(f"Search took {response.query_time_ms}ms")
```

### **Server Information**

```python
# Health check
health = client.health_check()
print(f"Server healthy: {health.healthy}")

# Server statistics
stats = client.get_server_stats()
print(f"Total vectors: {stats.total_vectors}")
print(f"Collections: {stats.total_collections}")
print(f"Memory usage: {stats.memory_usage} bytes")
print(f"Uptime: {stats.uptime_seconds}s")

# Quick connectivity test
is_reachable = client.ping()
print(f"Server reachable: {is_reachable}")

# Comprehensive info
info = client.get_info()
print("Client info:", info["client"])
print("Server info:", info["server"])
```

## 🧪 **Advanced Examples**

### **Working with NumPy Arrays**

```python
import numpy as np
from vectordb_client import VectorDBClient
from vectordb_client.types import Vector

client = VectorDBClient()

# Create collection for embeddings
client.create_collection_simple("embeddings", 384, "cosine")

# Work directly with NumPy arrays
embeddings = np.random.random((1000, 384))
ids = [f"embedding_{i}" for i in range(1000)]
metadata_list = [{"index": i, "batch": i // 100} for i in range(1000)]

# Batch insert using NumPy
vectors = [
    Vector.from_numpy(id=ids[i], data=embeddings[i], metadata=metadata_list[i])
    for i in range(len(embeddings))
]

# Insert in batches
batch_size = 100
for i in range(0, len(vectors), batch_size):
    batch = vectors[i:i + batch_size]
    response = client.insert_vectors("embeddings", batch)
    print(f"Inserted batch {i // batch_size + 1}: {response.inserted_count} vectors")

# Search with NumPy array
query_embedding = np.random.random(384)
results = client.search_simple("embeddings", query_embedding, limit=5)

# Convert results back to NumPy if needed
for result in results:
    vector = client.get_vector("embeddings", result.id)
    vector_array = vector.to_numpy()  # Convert to NumPy array
    print(f"Vector {result.id} shape: {vector_array.shape}")
```

### **Async Batch Processing**

```python
import asyncio
import numpy as np
from vectordb_client import AsyncVectorDBClient

async def process_large_dataset():
    async with AsyncVectorDBClient() as client:
        # Create collection
        await client.create_collection_simple("large_dataset", 512, "euclidean")
        
        # Generate large dataset
        num_vectors = 10000
        dimension = 512
        dataset = np.random.random((num_vectors, dimension))
        
        # Prepare batch data
        batch_data = [
            (f"vec_{i}", dataset[i], {"batch": i // 1000, "index": i})
            for i in range(num_vectors)
        ]
        
        # Concurrent insertion with progress tracking
        batch_size = 200
        max_concurrent = 20
        
        start_time = asyncio.get_event_loop().time()
        
        responses = await client.batch_insert_concurrent(
            collection_name="large_dataset",
            vectors_data=batch_data,
            batch_size=batch_size,
            max_concurrent_batches=max_concurrent
        )
        
        end_time = asyncio.get_event_loop().time()
        
        total_inserted = sum(r.inserted_count or 0 for r in responses)
        duration = end_time - start_time
        rate = total_inserted / duration
        
        print(f"Inserted {total_inserted} vectors in {duration:.2f}s")
        print(f"Rate: {rate:.2f} vectors/second")
        
        # Verify with search
        query_vector = np.random.random(512)
        results = await client.search_simple("large_dataset", query_vector, limit=10)
        print(f"Search found {len(results)} results")

# Run the async processing
asyncio.run(process_large_dataset())
```

### **Error Handling and Retries**

```python
import time
from vectordb_client import VectorDBClient
from vectordb_client.exceptions import (
    VectorDBError, ConnectionError, CollectionNotFoundError,
    VectorNotFoundError, RateLimitError
)

def robust_insert_with_retry(client, collection_name, vectors, max_retries=3):
    """Insert vectors with automatic retry on failure."""
    for attempt in range(max_retries):
        try:
            response = client.insert_vectors(collection_name, vectors)
            print(f"Successfully inserted {response.inserted_count} vectors")
            return response
            
        except RateLimitError as e:
            if attempt < max_retries - 1:
                wait_time = 2 ** attempt  # Exponential backoff
                print(f"Rate limited, waiting {wait_time}s before retry...")
                time.sleep(wait_time)
            else:
                raise e
                
        except ConnectionError as e:
            if attempt < max_retries - 1:
                print(f"Connection failed, retrying... ({attempt + 1}/{max_retries})")
                time.sleep(1)
            else:
                raise e
                
        except CollectionNotFoundError:
            print(f"Collection '{collection_name}' not found, creating...")
            client.create_collection_simple(collection_name, 128, "cosine")
            # Retry the insertion
            continue
            
    raise VectorDBError(f"Failed to insert after {max_retries} attempts")

# Usage
client = VectorDBClient()
vectors = [Vector(id=f"test_{i}", data=[0.1] * 128) for i in range(10)]

try:
    robust_insert_with_retry(client, "test_collection", vectors)
except VectorDBError as e:
    print(f"Final error: {e}")
```

### **Configuration and Connection Management**

```python
from vectordb_client import VectorDBClient
import os

# Configuration from environment variables
client = VectorDBClient(
    host=os.getenv("VECTORDB_HOST", "localhost"),
    port=int(os.getenv("VECTORDB_PORT", "8080")),
    ssl=os.getenv("VECTORDB_SSL", "false").lower() == "true",
    timeout=float(os.getenv("VECTORDB_TIMEOUT", "30.0"))
)

# Connection testing and fallback
def get_client_with_fallback():
    """Try multiple connection options."""
    
    # Try primary server
    try:
        primary_client = VectorDBClient(host="primary.vectordb.com", port=8080)
        if primary_client.ping():
            return primary_client
        primary_client.close()
    except Exception:
        pass
    
    # Try secondary server
    try:
        secondary_client = VectorDBClient(host="secondary.vectordb.com", port=8080)
        if secondary_client.ping():
            return secondary_client
        secondary_client.close()
    except Exception:
        pass
    
    # Fall back to localhost
    return VectorDBClient(host="localhost", port=8080)

# Context managers for resource cleanup
with get_client_with_fallback() as client:
    # Use client here - automatically closed when leaving context
    collections = client.list_collections()
    print(f"Available collections: {collections.collections}")
```

## 🧪 **Testing**

```bash
# Run unit tests
python -m pytest tests/

# Run with coverage
python -m pytest tests/ --cov=vectordb_client --cov-report=html

# Run integration tests (requires running d-vecDB server)
python -m pytest tests/integration/ -v

# Run performance benchmarks
python -m pytest tests/benchmarks/ -v
```

## 🔧 **Development**

```bash
# Setup development environment
git clone https://github.com/rdmurugan/d-vecDB.git
cd d-vecDB/python-client

# Install in development mode
pip install -e .[dev]

# Run code formatting
black vectordb_client/
isort vectordb_client/

# Run type checking  
mypy vectordb_client/

# Run linting
flake8 vectordb_client/
```

## 📊 **Performance Tips**

### **Batch Operations**
- Use `insert_vectors()` instead of multiple `insert_vector()` calls
- For async clients, use `batch_insert_concurrent()` for maximum throughput
- Optimal batch size is typically 100-1000 vectors depending on dimension

### **Connection Pooling**
- Async clients automatically pool HTTP connections
- Increase `connection_pool_size` for high-concurrency applications
- Reuse client instances instead of creating new ones

### **Search Optimization**
- Lower `ef_search` values for faster but less accurate search
- Use metadata filtering to reduce search space
- Consider the trade-off between speed and recall

### **Memory Management**
- Use NumPy arrays for large vector datasets
- Close clients explicitly or use context managers
- Monitor memory usage with large batch operations

## 🤝 **Contributing**

We welcome contributions! Please see our [Contributing Guide](../CONTRIBUTING.md) for details.

### **Development Setup**
1. Fork the repository
2. Create a feature branch
3. Install development dependencies: `pip install -e .[dev]`
4. Make changes and add tests
5. Run tests: `pytest`
6. Submit a pull request

## 📄 **License**

This project is licensed under the d-vecDB Enterprise License - see the [LICENSE](https://github.com/rdmurugan/d-vecDB/blob/master/LICENSE) file for details.

**For Enterprise Use**: Commercial usage requires a separate enterprise license. Contact durai@infinidatum.com for licensing terms.

## 🔗 **Links**

- **PyPI Package (Client)**: https://pypi.org/project/d-vecdb/
- **PyPI Package (Server)**: https://pypi.org/project/d-vecdb-server/
- **GitHub Repository**: https://github.com/rdmurugan/d-vecDB
- **Documentation**: https://github.com/rdmurugan/d-vecDB#readme
- **Binary Releases**: https://github.com/rdmurugan/d-vecDB/releases

## 🆘 **Support**

- **GitHub Issues**: [Report bugs or request features](https://github.com/rdmurugan/d-vecDB/issues)
- **GitHub Discussions**: [Community support and Q&A](https://github.com/rdmurugan/d-vecDB/discussions)
- **Email**: durai@infinidatum.com
- **Commercial Support**: Available for enterprise customers

## 🤝 **Related Packages**

- **[d-vecdb-server](https://pypi.org/project/d-vecdb-server/)** - Complete server package with embedded binaries (recommended for quick start)
- **[d-vecdb](https://pypi.org/project/d-vecdb/)** - Python client library (this package)

## 📈 **Version History**

See [CHANGELOG](https://github.com/rdmurugan/d-vecDB/releases) for version history and release notes.

**Current Version**: 0.2.0
- ✅ Published on PyPI
- ✅ Full type safety (py.typed marker)
- ✅ Production-ready with WAL protection
- ✅ GPU acceleration support
- ✅ Comprehensive documentation

---

**Built with ❤️ by the d-vecDB team**

*Star us on [GitHub](https://github.com/rdmurugan/d-vecDB) ⭐*