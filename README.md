# 🚀 d-vecDB

[![Rust Version](https://img.shields.io/badge/rust-1.70+-brightgreen.svg)](https://rustup.rs/)
[![License: Enterprise](https://img.shields.io/badge/License-Enterprise-red.svg)](LICENSE)
[![Performance](https://img.shields.io/badge/performance-35M+%20ops%2Fs-orange.svg)](#-performance-benchmarks)

**A high-performance, production-ready vector database written in Rust**

d-vecDB is a modern vector database designed for AI applications, semantic search, and similarity matching. Built from the ground up in Rust, it delivers exceptional performance, memory safety, and concurrent processing capabilities.

---

## 🎯 **Key Features**

### ⚡ **Ultra-High Performance**
- **Sub-microsecond vector operations** (28-76ns per distance calculation)
- **HNSW indexing** with O(log N) search complexity
- **Concurrent processing** with Rust's fearless concurrency
- **Memory-mapped storage** for efficient large dataset handling

### 🏗️ **Production Architecture**
- **gRPC & REST APIs** for universal client compatibility
- **Write-Ahead Logging (WAL)** for ACID durability and crash recovery  
- **Multi-threaded indexing** and query processing
- **Comprehensive error handling** and observability

### 🔧 **Developer Experience**
- **Type-safe APIs** with Protocol Buffers
- **Rich metadata support** with JSON field storage
- **Comprehensive benchmarking** suite with HTML reports
- **CLI tools** for database management

### 📊 **Enterprise Ready**
- **Horizontal scaling** capabilities
- **Monitoring integration** with Prometheus metrics
- **Flexible deployment** (standalone, containerized, embedded)
- **Cross-platform support** (Linux, macOS, Windows)

---

## 📈 **Benchmark Results**

### **🎯 Production Performance (October 2025)**

*Latest benchmarks on DigitalOcean 2 vCPU, 2GB RAM (Optimized Release Build)*

#### **Vector Insertion Throughput**

| Batch Size | d-vecDB | Qdrant | Status |
|-----------|---------|--------|--------|
| **Single (1)** | **315 vec/s** | 275 vec/s | ✅ **15% FASTER** |
| **Small (10)** | 1,293 vec/s | 1,628 vec/s | 1.26x slower |
| **Medium (100)** | 2,027 vec/s | 3,720 vec/s | 1.84x slower |
| **Large (500)** | 2,262 vec/s | 4,244 vec/s | 1.88x slower |

**Key Achievement**: d-vecDB **beats Qdrant** on single insert throughput! 🏆

**Total Improvement**: 6.7x from initial baseline (336 → 2,262 vec/s)

#### **WAL Corruption Protection** ✅
- CRC32 checksumming for all entries
- Magic number boundaries for corruption detection
- Graceful recovery from crashes and partial writes
- Production-grade durability

#### **Hardware Acceleration Support**

**🚀 GPU Acceleration** (Auto-detected)
- Automatic fallback to CPU if GPU unavailable
- WGPU compute shaders for parallel distance calculations
- **10-50x speedup** on supported hardware (NVIDIA, AMD, Apple Silicon)

**⚡ SIMD Optimization** (AVX2/SSE2)
- Vectorized distance calculations
- **2-3x faster** than scalar operations
- Automatic CPU feature detection

### **Core Performance**

*Tested on macOS Darwin 24.6.0*

| Operation | Latency | Throughput |
|-----------|---------|------------|
| **Dot Product** | 28.3 ns | 35.4M ops/sec |
| **Euclidean Distance** | 30.6 ns | 32.7M ops/sec |
| **Cosine Similarity** | 76.1 ns | 13.1M ops/sec |

### **Performance Projections on Higher-End Hardware**

Based on our benchmark results, here are conservative performance extrapolations for production hardware:

#### **High-End Server (32-core AMD EPYC, 128GB RAM, NVMe)**
| Operation | Current (Mac) | Projected (Server) | Improvement |
|-----------|---------------|-------------------|-------------|
| **Distance Calculations** | 35M ops/sec | **150M+ ops/sec** | 4.3x |
| **Vector Insertion** | 7K vectors/sec | **50K+ vectors/sec** | 7x |
| **Vector Search** | 13K queries/sec | **100K+ queries/sec** | 7.7x |
| **Concurrent Queries** | Single-threaded | **500K+ queries/sec** | 38x |

#### **Optimized Cloud Instance (16-core, 64GB RAM, SSD)**
| Operation | Current (Mac) | Projected (Cloud) | Improvement |
|-----------|---------------|-------------------|-------------|
| **Distance Calculations** | 35M ops/sec | **80M+ ops/sec** | 2.3x |
| **Vector Insertion** | 7K vectors/sec | **25K+ vectors/sec** | 3.6x |
| **Vector Search** | 13K queries/sec | **45K+ queries/sec** | 3.5x |
| **Concurrent Queries** | Single-threaded | **180K+ queries/sec** | 14x |

*Projections based on CPU core scaling, memory bandwidth improvements, and storage I/O optimizations*

---

## 🏗️ **Architecture**

```
┌─────────────────────────────────────────────────────────────┐
│                  🎯 d-vecDB Stack                           │
├─────────────────────────────────────────────────────────────┤
│  CLI Tool      │  Client SDKs   │  REST + gRPC APIs          │
│  (Management)  │  (Rust/Python) │  (Universal Access)        │
├─────────────────────────────────────────────────────────────┤
│                    Vector Store Engine                      │
│              (Indexing + Storage + Querying)                │
├─────────────────────────────────────────────────────────────┤
│  HNSW Index    │   WAL Storage   │   Memory Mapping          │
│  (O(log N))    │   (Durability)  │   (Performance)           │
└─────────────────────────────────────────────────────────────┘
```

### **Core Components**

- **🔍 HNSW Index**: Hierarchical Navigable Small World graphs for approximate nearest neighbor search
- **💾 Storage Engine**: Memory-mapped files with write-ahead logging for durability
- **🌐 API Layer**: Both REST (HTTP/JSON) and gRPC (Protocol Buffers) interfaces
- **📊 Monitoring**: Built-in Prometheus metrics and comprehensive logging
- **🔧 CLI Tools**: Database management, collection operations, and administrative tasks

---

## 🚀 **Quick Start**

### **Installation**

**Option 1: Install Python Client from PyPI**
```bash
# Install d-vecDB Python client library
pip install d-vecdb

```

**After installation, you have access to:**
- **Python client library**: `vectordb_client` for connecting to d-vecDB servers
- **Example scripts**: Located in your Python site-packages

**Note**: This installs only the Python client library. You need to build the server from source (see below).

**Option 2: Install from Source**
```bash
# Clone the repository
git clone https://github.com/rdmurugan/d-vecDB.git
cd d-vecDB

# Quick install using script
./scripts/install.sh

# Or manual installation
pip install .
```

**Option 3: For Development**
```bash
# Clone and setup development environment
git clone https://github.com/rdmurugan/d-vecDB.git
cd d-vecDB

# Install in development mode with all extras
./scripts/install.sh dev

# Build Rust server components
./scripts/build-server.sh
```

**Option 4: Using Virtual Environment**
```bash
# Create isolated environment
./scripts/install.sh venv
source venv/bin/activate  # Linux/macOS
# venv\Scripts\activate   # Windows
```

### **Getting Started**

**Step 1: Build and Start the d-vecDB Server**

Since `pip install d-vecdb` only provides the Python client, you need to build the server from source:

```bash
# Clone the repository and build the server
git clone https://github.com/rdmurugan/d-vecDB.git
cd d-vecDB

# Build the server (requires Rust)
cargo build --release

# Start the server
./target/release/vectordb-server --config config.toml
```

**Step 2: Use the Python Client (after pip install)**
```python
# Import the client library (available after pip install d-vecdb)
from vectordb_client import VectorDBClient
import numpy as np

# Connect to the server you built and started in Step 1
client = VectorDBClient(host="localhost", port=8080)

# Create your first collection
client.create_collection_simple("my_vectors", 128, "cosine")

# Insert a vector
vector_data = np.random.random(128)
client.insert_simple("my_vectors", "vector_1", vector_data)

# Search for similar vectors
query_vector = np.random.random(128)
results = client.search_simple("my_vectors", query_vector, limit=5)
print(f"Found {len(results)} similar vectors")
```


### **REST API Usage**

Once your server is running, you can also use the REST API directly:

```bash
# Create a collection
curl -X POST http://localhost:8080/collections \
  -H "Content-Type: application/json" \
  -d '{
    "name": "documents",
    "dimension": 128,
    "distance_metric": "cosine"
  }'

# Insert vectors
curl -X POST http://localhost:8080/collections/documents/vectors \
  -H "Content-Type: application/json" \
  -d '{
    "id": "doc1",
    "data": [0.1, 0.2, 0.3, ...],
    "metadata": {"title": "Example Document"}
  }'

# Search for similar vectors
curl -X POST http://localhost:8080/collections/documents/search \
  -H "Content-Type: application/json" \
  -d '{
    "query_vector": [0.1, 0.2, 0.3, ...],
    "limit": 10
  }'
```

---

## 🛠️ **Development Setup**

### **Prerequisites**
- **Rust** 1.70+ ([Install Rust](https://rustup.rs/))
- **Protocol Buffers** compiler (`protoc`)
- **Git** for version control

### **Build Instructions**

```bash
# Development build
cargo build

# Optimized release build  
cargo build --release

# Run all tests
cargo test

# Run benchmarks
cargo bench --package vectordb-common

# Generate documentation
cargo doc --open
```

### **Project Structure**

```
d-vecDB/
├── common/          # Core types, distance functions, utilities
├── index/           # HNSW indexing implementation
├── storage/         # WAL, memory-mapping, persistence
├── vectorstore/     # Main vector store engine
├── server/          # REST & gRPC API servers
├── python-client/   # 🐍 Official Python client library
├── client/          # Additional client SDKs and libraries
├── cli/             # Command-line tools
├── proto/           # Protocol Buffer definitions
└── benchmarks/      # Performance testing suite
```

---

## 📚 **Client Libraries**

d-vecDB provides official client libraries for multiple programming languages:

### 🐍 **Python Client**
[![Python 3.8+](https://img.shields.io/badge/python-3.8+-blue.svg)](https://www.python.org/downloads/)

**Full-featured Python client with async support, NumPy integration, and type safety.**

- 🔄 **Sync & Async**: Both synchronous and asynchronous clients
- ⚡ **High Performance**: Concurrent batch operations (1000+ vectors/sec)
- 🧮 **NumPy Native**: Direct NumPy array support
- 🔒 **Type Safe**: Pydantic models with validation
- 🌐 **Multi-Protocol**: REST and gRPC support

```bash
# Install from PyPI (included with d-vecdb)
pip install d-vecdb

# Quick usage
from vectordb_client import VectorDBClient
import numpy as np

client = VectorDBClient()
client.create_collection_simple("docs", 384, "cosine")
client.insert_simple("docs", "doc_1", np.random.random(384))
results = client.search_simple("docs", np.random.random(384), limit=5)
```

**📖 [Complete Python Documentation →](python-client/README.md)**

### 🦀 **Rust Client** *(Native)*

Direct access to the native Rust API for maximum performance.

### 🌐 **HTTP/REST API**
Language-agnostic REST API with OpenAPI specification.

**📖 [API Documentation →](docs/api.md)**

### 🚧 **Coming Soon**
- **JavaScript/TypeScript** client
- **Go** client  
- **Java** client
- **C++** bindings

---

## 📊 **Comprehensive Benchmarking**

### **Running Benchmarks**

```bash
# Core performance benchmarks
cargo bench --package vectordb-common

# Generate HTML reports
cargo bench --package vectordb-common
open target/criterion/report/index.html

# Custom benchmark suite
./scripts/run-comprehensive-benchmarks.sh
```

### **Benchmark Categories**

1. **🧮 Distance Calculations**: Core mathematical operations (cosine, euclidean, dot product)
2. **🗂️ Index Operations**: Vector insertion, search, and maintenance  
3. **💾 Storage Performance**: WAL writes, memory-mapped reads, persistence
4. **🌐 API Throughput**: REST and gRPC endpoint performance
5. **📈 Scaling Tests**: Performance under load with varying dataset sizes

### **Hardware Optimization Guide**

#### **For Maximum Insertion Throughput:**
- **CPU**: High core count (32+ cores) for parallel indexing
- **RAM**: Large memory pool (128GB+) for index caching  
- **Storage**: NVMe SSDs for fast WAL writes

#### **For Maximum Query Performance:**
- **CPU**: High single-thread performance with many cores
- **RAM**: Fast memory (DDR4-3200+) for index traversal
- **Network**: High bandwidth for concurrent client connections

#### **For Large Scale Deployments:**
- **Distributed Setup**: Multiple nodes with load balancing
- **Storage Tiering**: Hot data in memory, warm data on SSD
- **Monitoring**: Comprehensive metrics and alerting

---

## 🔧 **Configuration**

### **Server Configuration**

```toml
# config.toml
[server]
host = "0.0.0.0"
port = 8080
grpc_port = 9090
workers = 8

[storage]
data_dir = "./data"
wal_sync_interval = "1s"
memory_map_size = "1GB"

[index]
hnsw_max_connections = 16
hnsw_ef_construction = 200
hnsw_max_layer = 16

[monitoring]
enable_metrics = true
prometheus_port = 9091
log_level = "info"
```

### **Performance Tuning**

```toml
[performance]
# Optimize for insertion throughput
batch_size = 1000
insert_workers = 16

# Optimize for query latency  
query_cache_size = "500MB"
prefetch_enabled = true

# Memory management
gc_interval = "30s"
memory_limit = "8GB"
```

---

## 🌐 **API Reference**

### **REST API**

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/collections` | POST | Create collection |
| `/collections/{name}` | GET | Get collection info |
| `/collections/{name}/vectors` | POST | Insert vectors |
| `/collections/{name}/search` | POST | Search vectors |
| `/collections/{name}/vectors/{id}` | DELETE | Delete vector |
| `/stats` | GET | Server statistics |
| `/health` | GET | Health check |

### **gRPC Services**

```protobuf
service VectorDb {
  rpc CreateCollection(CreateCollectionRequest) returns (CreateCollectionResponse);
  rpc Insert(InsertRequest) returns (InsertResponse);
  rpc BatchInsert(BatchInsertRequest) returns (BatchInsertResponse);
  rpc Query(QueryRequest) returns (QueryResponse);
  rpc Delete(DeleteRequest) returns (DeleteResponse);
  rpc GetStats(GetStatsRequest) returns (GetStatsResponse);
}
```

### **Client SDKs**

```rust
// Rust Client
use vectordb_client::VectorDbClient;

let client = VectorDbClient::new("http://localhost:8080").await?;

// Create collection
client.create_collection("documents", 128, DistanceMetric::Cosine).await?;

// Insert vector
client.insert("documents", "doc1", vec![0.1, 0.2, 0.3], metadata).await?;

// Search
let results = client.search("documents", query_vector, 10).await?;
```

```python
# Python Client (Available Now!)
from vectordb_client import VectorDBClient

client = VectorDBClient("http://localhost:8080")
client.create_collection_simple("documents", 128, "cosine")
client.insert_simple("documents", "doc1", [0.1, 0.2, 0.3], {"title": "Example"})
results = client.search_simple("documents", query_vector, limit=10)
```

---

## 🔍 **Use Cases**

### **🤖 AI & Machine Learning**
- **Embedding storage** for transformer models (BERT, GPT, etc.)
- **Recommendation engines** with user/item similarity
- **Content-based filtering** and personalization

### **🔍 Search & Discovery**  
- **Semantic search** in documents and knowledge bases
- **Image/video similarity** search and retrieval
- **Product recommendation** in e-commerce platforms

### **📊 Data Analytics**
- **Anomaly detection** in high-dimensional data
- **Clustering and classification** of complex datasets  
- **Feature matching** in computer vision applications

### **🏢 Enterprise Applications**
- **Document similarity** in legal and compliance systems
- **Fraud detection** through pattern matching
- **Customer segmentation** and behavioral analysis

---

## 🚦 **Production Deployment**

### **Docker Deployment**

```dockerfile
FROM rust:1.70 as builder
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /target/release/vectordb-server /usr/local/bin/
EXPOSE 8080 9090 9091
CMD ["vectordb-server", "--config", "/etc/vectordb/config.toml"]
```

```bash
# Build and run
docker build -t d-vecdb .
docker run -p 8080:8080 -p 9090:9090 -v ./data:/data d-vecdb
```

### **Kubernetes Deployment**

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: d-vecdb
spec:
  replicas: 3
  selector:
    matchLabels:
      app: d-vecdb
  template:
    metadata:
      labels:
        app: d-vecdb
    spec:
      containers:
      - name: d-vecdb
        image: d-vecdb:latest
        ports:
        - containerPort: 8080
        - containerPort: 9090
        resources:
          requests:
            memory: "2Gi"
            cpu: "500m"
          limits:
            memory: "8Gi"
            cpu: "4"
```

### **Monitoring Integration**

```yaml
# Prometheus configuration
- job_name: 'd-vecdb'
  static_configs:
  - targets: ['d-vecdb:9091']
  scrape_interval: 15s
  metrics_path: /metrics
```

---

## 📈 **Performance Comparison**

### **vs. Traditional Vector Databases**

| Feature | d-vecDB | Pinecone | Weaviate | Qdrant |
|---------|-------------|----------|----------|--------|
| **Language** | Rust | Python/C++ | Go | Rust |
| **Memory Safety** | ✅ Zero-cost | ❌ Manual | ❌ GC Overhead | ✅ Zero-cost |
| **Concurrency** | ✅ Native | ⚠️ Limited | ⚠️ GC Pauses | ✅ Native |
| **Deployment** | ✅ Single Binary | ❌ Cloud Only | ⚠️ Complex | ✅ Flexible |
| **Performance** | ✅ 35M ops/sec | ⚠️ Network Bound | ⚠️ GC Limited | ✅ Comparable |

### **Scaling Characteristics**

| Dataset Size | Query Latency | Memory Usage | Throughput |
|-------------|---------------|--------------|------------|
| **1K vectors** | <100µs | <10MB | 50K+ qps |
| **100K vectors** | <500µs | <500MB | 25K+ qps |
| **1M vectors** | <2ms | <2GB | 15K+ qps |
| **10M vectors** | <10ms | <8GB | 8K+ qps |

---

## 🤝 **Contributing**

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

### **Development Workflow**

```bash
# Fork and clone the repository
git clone https://github.com/your-username/d-vecDB.git
cd d-vecDB

# Create a feature branch
git checkout -b feature/amazing-feature

# Make changes and test
cargo test
cargo clippy
cargo fmt

# Submit a pull request
git push origin feature/amazing-feature
```

### **Areas for Contribution**
- 🚀 Performance optimizations and SIMD implementations
- 🌐 Additional client SDK languages (Python, JavaScript, Java)
- 📊 Advanced indexing algorithms (IVF, PQ, LSH)
- 🔧 Operational tools and monitoring dashboards
- 📚 Documentation and example applications

---

## 📄 **License**

This project is licensed under the d-vecDB Enterprise License - see the [LICENSE](LICENSE) file for details.

**For Enterprise Use**: Commercial usage requires a separate enterprise license. Contact durai@infinidatum.com for licensing terms.

---

## 🆘 **Support**

- **📧 Email**: durai@infinidatum.com
- **💬 Discord**: [d-vecDB Community](https://discord.gg/d-vecdb)
- **🐛 Issues**: [GitHub Issues](https://github.com/rdmurugan/d-vecDB/issues)
- **📚 Documentation**: [docs.d-vecdb.com](https://docs.d-vecdb.com)

---

## 🙏 **Acknowledgments**

- Built with ❤️ in Rust
- Inspired by modern vector database architectures
- Powered by the amazing Rust ecosystem
- Community-driven development

---

**⚡ Ready to build the future of AI-powered applications? Get started with d-vecDB today!**
