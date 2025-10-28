#!/usr/bin/env python3
"""
d-vecDB benchmark client.
"""

import sys
from pathlib import Path

# Add python-client to path
sys.path.insert(0, str(Path(__file__).parent.parent.parent / "python-client"))

import numpy as np
from typing import Dict, List, Any, Optional
from benchmark_base import VectorDBBenchmark
from tqdm import tqdm


class DVecDBBenchmark(VectorDBBenchmark):
    """d-vecDB benchmark implementation"""

    def __init__(self, config: Dict[str, Any]):
        super().__init__("d-vecDB", config)
        self.client = None
        self.db_config = config.get('dvecdb', {})

    def connect(self):
        """Connect to d-vecDB"""
        from vectordb_client import VectorDBClient

        self.client = VectorDBClient(
            host=self.db_config.get('host', 'localhost'),
            port=self.db_config.get('port', 8080)
        )

        # Test connection
        try:
            self.client.ping()
            print(f"✅ Connected to d-vecDB at {self.db_config.get('host')}:{self.db_config.get('port')}")
        except Exception as e:
            print(f"❌ Failed to connect to d-vecDB: {e}")
            raise

    def disconnect(self):
        """Disconnect from d-vecDB"""
        if self.client:
            self.client.close()
            self.client = None

    def create_collection(self, name: str, dimension: int):
        """Create a collection"""
        try:
            # Delete if exists
            try:
                self.client.delete_collection(name)
            except:
                pass

            # Create new
            self.client.create_collection_simple(
                name=name,
                dimension=dimension,
                distance_metric="cosine"
            )

            print(f"✅ Created collection: {name} (dimension={dimension})")

        except Exception as e:
            print(f"❌ Failed to create collection: {e}")
            raise

    def delete_collection(self, name: str):
        """Delete a collection"""
        try:
            self.client.delete_collection(name)
        except Exception as e:
            # Ignore if doesn't exist
            pass

    def insert_vectors(
        self,
        collection: str,
        vectors: np.ndarray,
        metadata: Optional[List[Dict[str, Any]]] = None,
        batch_size: int = 100
    ):
        """Insert vectors in batches"""
        num_vectors = len(vectors)
        num_batches = (num_vectors + batch_size - 1) // batch_size

        with tqdm(total=num_vectors, desc=f"Inserting to {self.name}") as pbar:
            for i in range(num_batches):
                start_idx = i * batch_size
                end_idx = min(start_idx + batch_size, num_vectors)

                batch_vectors = vectors[start_idx:end_idx]
                batch_metadata = metadata[start_idx:end_idx] if metadata else None

                # Insert batch
                for j, vector in enumerate(batch_vectors):
                    vector_id = f"vec_{start_idx + j}"
                    meta = batch_metadata[j] if batch_metadata else None

                    self.client.insert_simple(
                        collection_name=collection,
                        vector_id=vector_id,
                        vector_data=vector,
                        metadata=meta
                    )

                pbar.update(len(batch_vectors))

    def search_vectors(
        self,
        collection: str,
        query_vectors: np.ndarray,
        top_k: int = 10
    ) -> List[List[Dict[str, Any]]]:
        """Search for similar vectors"""
        results = []

        for query in query_vectors:
            result = self.client.search_simple(
                collection_name=collection,
                query_vector=query,
                limit=top_k
            )
            results.append(result)

        return results


if __name__ == "__main__":
    # Test the benchmark client
    import yaml
    from data_generator import DataGenerator

    # Load config
    config_path = Path(__file__).parent / "config.yaml"
    with open(config_path) as f:
        config = yaml.safe_load(f)

    # Create benchmark instance
    benchmark = DVecDBBenchmark(config['databases'])

    try:
        # Connect
        benchmark.connect()

        # Generate test data
        print("\nGenerating test data...")
        generator = DataGenerator(dimension=128)
        vectors, _ = generator.generate_clustered_vectors(count=1000)
        metadata = generator.generate_metadata(len(vectors))
        query_vectors, _ = generator.generate_clustered_vectors(count=100)

        # Test insert
        print("\nTesting insert...")
        result = benchmark.benchmark_insert(
            dataset_name="test",
            vectors=vectors,
            metadata=metadata,
            batch_size=100
        )

        print(f"\nInsert Results:")
        print(f"  Throughput: {result.throughput:.2f} vectors/sec")
        print(f"  Duration: {result.duration_seconds:.2f} seconds")
        print(f"  Memory: {result.memory_mb:.2f} MB")

        # Test search
        print("\nTesting search...")
        result = benchmark.benchmark_search(
            dataset_name="test",
            query_vectors=query_vectors,
            top_k=10,
            warmup=10
        )

        print(f"\nSearch Results:")
        print(f"  Throughput: {result.throughput:.2f} queries/sec")
        print(f"  P50 Latency: {result.latency_p50:.2f} ms")
        print(f"  P99 Latency: {result.latency_p99:.2f} ms")

        # Cleanup
        benchmark.delete_collection("bench_test")

    finally:
        benchmark.disconnect()
