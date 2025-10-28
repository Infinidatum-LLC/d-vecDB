#!/usr/bin/env python3
"""
Base benchmark class for all vector database benchmarks.
"""

import time
import psutil
import numpy as np
from typing import Dict, List, Any, Optional
from dataclasses import dataclass
from abc import ABC, abstractmethod
import statistics


@dataclass
class BenchmarkResult:
    """Result from a single benchmark run"""
    database: str
    operation: str
    dataset: str
    count: int
    duration_seconds: float
    throughput: float
    latency_p50: float
    latency_p90: float
    latency_p95: float
    latency_p99: float
    memory_mb: float
    cpu_percent: float
    metadata: Dict[str, Any]


class VectorDBBenchmark(ABC):
    """Base class for vector database benchmarks"""

    def __init__(self, name: str, config: Dict[str, Any]):
        self.name = name
        self.config = config
        self.process = psutil.Process()

    @abstractmethod
    def connect(self):
        """Connect to the database"""
        pass

    @abstractmethod
    def disconnect(self):
        """Disconnect from the database"""
        pass

    @abstractmethod
    def create_collection(self, name: str, dimension: int):
        """Create a collection/index"""
        pass

    @abstractmethod
    def delete_collection(self, name: str):
        """Delete a collection/index"""
        pass

    @abstractmethod
    def insert_vectors(
        self,
        collection: str,
        vectors: np.ndarray,
        metadata: Optional[List[Dict[str, Any]]] = None,
        batch_size: int = 100
    ):
        """Insert vectors in batches"""
        pass

    @abstractmethod
    def search_vectors(
        self,
        collection: str,
        query_vectors: np.ndarray,
        top_k: int = 10
    ) -> List[List[Dict[str, Any]]]:
        """Search for similar vectors"""
        pass

    def measure_memory(self) -> float:
        """Get current memory usage in MB"""
        mem_info = self.process.memory_info()
        return mem_info.rss / 1024 / 1024

    def measure_cpu(self) -> float:
        """Get current CPU usage percentage"""
        return self.process.cpu_percent(interval=0.1)

    def benchmark_insert(
        self,
        dataset_name: str,
        vectors: np.ndarray,
        metadata: List[Dict[str, Any]],
        batch_size: int = 100
    ) -> BenchmarkResult:
        """Benchmark insert performance"""
        collection_name = f"bench_{dataset_name}"

        # Create collection
        dimension = vectors.shape[1]
        self.create_collection(collection_name, dimension)

        # Measure
        start_time = time.time()
        start_memory = self.measure_memory()

        self.insert_vectors(
            collection=collection_name,
            vectors=vectors,
            metadata=metadata,
            batch_size=batch_size
        )

        duration = time.time() - start_time
        end_memory = self.measure_memory()
        cpu_usage = self.measure_cpu()

        throughput = len(vectors) / duration

        return BenchmarkResult(
            database=self.name,
            operation="insert",
            dataset=dataset_name,
            count=len(vectors),
            duration_seconds=duration,
            throughput=throughput,
            latency_p50=0,  # N/A for batch insert
            latency_p90=0,
            latency_p95=0,
            latency_p99=0,
            memory_mb=end_memory - start_memory,
            cpu_percent=cpu_usage,
            metadata={"batch_size": batch_size}
        )

    def benchmark_search(
        self,
        dataset_name: str,
        query_vectors: np.ndarray,
        top_k: int = 10,
        warmup: int = 100
    ) -> BenchmarkResult:
        """Benchmark search performance"""
        collection_name = f"bench_{dataset_name}"

        # Warmup
        if warmup > 0:
            warmup_queries = query_vectors[:min(warmup, len(query_vectors))]
            for query in warmup_queries:
                self.search_vectors(collection_name, query.reshape(1, -1), top_k)

        # Measure individual query latencies
        latencies = []
        start_memory = self.measure_memory()

        for query in query_vectors:
            start = time.time()
            self.search_vectors(collection_name, query.reshape(1, -1), top_k)
            latency = (time.time() - start) * 1000  # Convert to ms
            latencies.append(latency)

        end_memory = self.measure_memory()
        cpu_usage = self.measure_cpu()

        # Calculate statistics
        total_duration = sum(latencies) / 1000  # Convert back to seconds
        throughput = len(query_vectors) / total_duration

        return BenchmarkResult(
            database=self.name,
            operation="search",
            dataset=dataset_name,
            count=len(query_vectors),
            duration_seconds=total_duration,
            throughput=throughput,
            latency_p50=np.percentile(latencies, 50),
            latency_p90=np.percentile(latencies, 90),
            latency_p95=np.percentile(latencies, 95),
            latency_p99=np.percentile(latencies, 99),
            memory_mb=end_memory - start_memory,
            cpu_percent=cpu_usage,
            metadata={"top_k": top_k, "queries": len(query_vectors)}
        )

    def benchmark_concurrent_search(
        self,
        dataset_name: str,
        query_vectors: np.ndarray,
        top_k: int = 10,
        concurrent: int = 10
    ) -> BenchmarkResult:
        """Benchmark concurrent search performance"""
        from concurrent.futures import ThreadPoolExecutor, as_completed

        collection_name = f"bench_{dataset_name}"

        # Split queries among threads
        queries_per_thread = len(query_vectors) // concurrent

        def search_thread(queries):
            latencies = []
            for query in queries:
                start = time.time()
                self.search_vectors(collection_name, query.reshape(1, -1), top_k)
                latency = (time.time() - start) * 1000
                latencies.append(latency)
            return latencies

        start_time = time.time()
        start_memory = self.measure_memory()

        all_latencies = []
        with ThreadPoolExecutor(max_workers=concurrent) as executor:
            futures = []
            for i in range(concurrent):
                start_idx = i * queries_per_thread
                end_idx = start_idx + queries_per_thread
                thread_queries = query_vectors[start_idx:end_idx]
                futures.append(executor.submit(search_thread, thread_queries))

            for future in as_completed(futures):
                all_latencies.extend(future.result())

        duration = time.time() - start_time
        end_memory = self.measure_memory()
        cpu_usage = self.measure_cpu()

        throughput = len(all_latencies) / duration

        return BenchmarkResult(
            database=self.name,
            operation="concurrent_search",
            dataset=dataset_name,
            count=len(all_latencies),
            duration_seconds=duration,
            throughput=throughput,
            latency_p50=np.percentile(all_latencies, 50),
            latency_p90=np.percentile(all_latencies, 90),
            latency_p95=np.percentile(all_latencies, 95),
            latency_p99=np.percentile(all_latencies, 99),
            memory_mb=end_memory - start_memory,
            cpu_percent=cpu_usage,
            metadata={
                "top_k": top_k,
                "concurrent": concurrent,
                "queries": len(all_latencies)
            }
        )


class BenchmarkRunner:
    """Run benchmarks across multiple databases"""

    def __init__(self, config: Dict[str, Any]):
        self.config = config
        self.results: List[BenchmarkResult] = []

    def add_result(self, result: BenchmarkResult):
        """Add a benchmark result"""
        self.results.append(result)

    def get_results(self) -> List[BenchmarkResult]:
        """Get all results"""
        return self.results

    def export_results(self, output_path: str):
        """Export results to JSON"""
        import json

        results_dict = [
            {
                "database": r.database,
                "operation": r.operation,
                "dataset": r.dataset,
                "count": r.count,
                "duration_seconds": r.duration_seconds,
                "throughput": r.throughput,
                "latency_p50": r.latency_p50,
                "latency_p90": r.latency_p90,
                "latency_p95": r.latency_p95,
                "latency_p99": r.latency_p99,
                "memory_mb": r.memory_mb,
                "cpu_percent": r.cpu_percent,
                "metadata": r.metadata
            }
            for r in self.results
        ]

        with open(output_path, 'w') as f:
            json.dump(results_dict, f, indent=2)

        print(f"✅ Results exported to {output_path}")
