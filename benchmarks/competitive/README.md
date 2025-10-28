# Competitive Benchmarking Suite

Comprehensive benchmarks comparing d-vecDB against Pinecone and Qdrant.

## Overview

This benchmarking suite provides **fair, reproducible, and comprehensive** performance comparisons across three vector databases:

- **d-vecDB** - Our blazing-fast Rust-based vector database
- **Pinecone** - Leading cloud-based vector database
- **Qdrant** - Popular open-source vector database

## What We Benchmark

### 1. **Insert Performance**
- Batch insert throughput (vectors/second)
- Different batch sizes (1, 10, 100, 500, 1000)
- Memory usage during insert

### 2. **Search Performance**
- Query latency (P50, P90, P95, P99)
- Query throughput (queries/second)
- Different top-K values (1, 5, 10, 50, 100)

### 3. **Concurrent Performance**
- Multiple concurrent clients (1, 10, 50, 100)
- Throughput under load
- Latency distribution under concurrency

### 4. **Memory Usage**
- Memory consumption per operation
- Memory efficiency

## Datasets

### Small (SIFT-like)
- **Vectors**: 10,000
- **Dimension**: 128
- **Queries**: 1,000
- **Use case**: Small-scale similarity search

### Medium (GloVe-like)
- **Vectors**: 100,000
- **Dimension**: 300
- **Queries**: 1,000
- **Use case**: Word embeddings, NLP

### Large (OpenAI embeddings)
- **Vectors**: 1,000,000
- **Dimension**: 1,536
- **Queries**: 1,000
- **Use case**: Large-scale semantic search

### Realistic (BERT embeddings)
- **Vectors**: 500,000
- **Dimension**: 768
- **Queries**: 5,000
- **Use case**: Production workload simulation

## Setup

### 1. Install Dependencies

```bash
cd benchmarks/competitive
pip install -r requirements.txt
```

### 2. Start d-vecDB Server

```bash
# From repo root
cargo build --release
./target/release/vectordb-server --host 0.0.0.0 --port 8080
```

### 3. Start Qdrant (Docker)

```bash
docker run -p 6333:6333 -p 6334:6334 \
  qdrant/qdrant
```

### 4. Set Pinecone API Key (Optional)

```bash
export PINECONE_API_KEY=your_api_key_here
```

## Running Benchmarks

### Quick Test (d-vecDB and Qdrant, small dataset)

```bash
python run_benchmarks.py \
  --databases dvecdb qdrant \
  --datasets small
```

### Full Benchmark (All databases, realistic dataset)

```bash
python run_benchmarks.py \
  --databases dvecdb pinecone qdrant \
  --datasets realistic
```

### Multiple Datasets

```bash
python run_benchmarks.py \
  --databases dvecdb qdrant \
  --datasets small medium large
```

### Single Database Test

```bash
# Test d-vecDB only
python benchmark_dvecdb.py

# Test Qdrant only
python benchmark_qdrant.py

# Test Pinecone only (requires API key)
python benchmark_pinecone.py
```

## Generate Test Data

```bash
python data_generator.py
```

This generates all datasets defined in `config.yaml`.

## Output

### Results Directory

```
results/
├── benchmark_results_20251028_143022.json  # Raw results
├── benchmark_report_20251028_143022.html   # HTML report
└── plots/                                   # Performance charts
    ├── insert_throughput.png
    ├── search_latency.png
    ├── concurrent_throughput.png
    └── memory_usage.png
```

### Results Format

```json
{
  "database": "d-vecDB",
  "operation": "search",
  "dataset": "realistic",
  "count": 5000,
  "duration_seconds": 6.75,
  "throughput": 740.74,
  "latency_p50": 1.35,
  "latency_p90": 1.87,
  "latency_p95": 2.15,
  "latency_p99": 2.89,
  "memory_mb": 45.2,
  "cpu_percent": 67.3,
  "metadata": {
    "top_k": 10,
    "queries": 5000
  }
}
```

## Expected Results (Predictions)

Based on d-vecDB's current performance:

### Insert Throughput (vectors/second)

| Database | Batch Size 1 | Batch Size 100 | Batch Size 1000 |
|----------|-------------|----------------|-----------------|
| **d-vecDB** | 5,000 | 20,000 | 50,000 |
| **Qdrant** | 3,000 | 12,000 | 30,000 |
| **Pinecone** | 1,500 | 5,000 | 8,000 |

### Search Latency P50 (milliseconds)

| Database | Top-K 1 | Top-K 10 | Top-K 100 |
|----------|---------|----------|-----------|
| **d-vecDB** | 0.85 | 1.35 | 2.50 |
| **Qdrant** | 1.50 | 2.80 | 5.00 |
| **Pinecone** | 50.0 | 65.0 | 100.0 |

*Note: Pinecone includes network latency (cloud-based)*

### Concurrent Throughput (queries/second)

| Database | 1 Client | 10 Clients | 100 Clients |
|----------|----------|------------|-------------|
| **d-vecDB** | 740 | 7,400 | 40,000 |
| **Qdrant** | 500 | 4,500 | 20,000 |
| **Pinecone** | 100 | 800 | 3,000 |

## Configuration

Edit `config.yaml` to customize:

- Dataset sizes and dimensions
- Batch sizes
- Top-K values
- Concurrent client counts
- Benchmark iterations
- Output settings

## Fair Benchmarking Guidelines

### ✅ What We Do

1. **Same Hardware**: All local databases run on the same machine
2. **Same Data**: Identical datasets for all databases
3. **Warmup**: All benchmarks include warmup queries
4. **Multiple Runs**: Average over multiple iterations
5. **Realistic Data**: Clustered vectors, not random
6. **Metadata**: Include metadata in all inserts
7. **Proper Cleanup**: Delete collections between runs

### ❌ What We Avoid

1. **Cherry-picking**: Report all results, not just favorable ones
2. **Unfair Configs**: Use default/recommended settings
3. **Cache Warming**: Clear caches between runs
4. **Network Bias**: Note that Pinecone includes network latency
5. **Tiny Datasets**: Use realistic dataset sizes

## Troubleshooting

### d-vecDB Connection Failed

```bash
# Make sure server is running
./target/release/vectordb-server --host 0.0.0.0 --port 8080
```

### Qdrant Connection Failed

```bash
# Start Qdrant with Docker
docker run -p 6333:6333 qdrant/qdrant
```

### Pinecone Authentication Failed

```bash
# Set API key
export PINECONE_API_KEY=your_key_here
```

### Memory Issues

For large datasets, increase available memory or reduce dataset size in `config.yaml`.

## Generating Reports

After running benchmarks:

```bash
# Generate HTML report with visualizations
python generate_report.py results/benchmark_results_*.json
```

This creates:
- HTML report with tables and charts
- Individual PNG charts
- Comparison tables

## Contributing

Improvements to the benchmarking suite are welcome:

1. Add new metrics
2. Add new databases
3. Improve visualization
4. Add statistical analysis
5. Improve fairness

## Citation

If you use these benchmarks in research or publications:

```bibtex
@misc{dvecdb-benchmarks,
  title={d-vecDB Competitive Benchmarks},
  author={d-vecDB Contributors},
  year={2025},
  url={https://github.com/rdmurugan/d-vecDB}
}
```

---

**Last Updated**: 2025-10-28
**Benchmark Version**: 1.0

Happy benchmarking! 🚀
