#!/usr/bin/env python3
"""
Qdrant benchmark client.
"""

import numpy as np
from typing import Dict, List, Any, Optional
from benchmark_base import VectorDBBenchmark
from tqdm import tqdm


class QdrantBenchmark(VectorDBBenchmark):
    """Qdrant benchmark implementation"""

    def __init__(self, config: Dict[str, Any]):
        super().__init__("Qdrant", config)
        self.client = None
        self.db_config = config.get('qdrant', {})

    def connect(self):
        """Connect to Qdrant"""
        try:
            from qdrant_client import QdrantClient
        except ImportError:
            print("❌ Qdrant client not installed. Install with: pip install qdrant-client")
            raise

        self.client = QdrantClient(
            host=self.db_config.get('host', 'localhost'),
            port=self.db_config.get('port', 6333)
        )

        print(f"✅ Connected to Qdrant at {self.db_config.get('host')}:{self.db_config.get('port')}")

    def disconnect(self):
        """Disconnect from Qdrant"""
        self.client = None

    def create_collection(self, name: str, dimension: int):
        """Create a Qdrant collection"""
        from qdrant_client.models import Distance, VectorParams

        collection_name = self.db_config.get('collection_name', 'benchmark_test')

        # Delete if exists
        try:
            self.client.delete_collection(collection_name)
        except:
            pass

        # Create new collection
        self.client.create_collection(
            collection_name=collection_name,
            vectors_config=VectorParams(
                size=dimension,
                distance=Distance.COSINE
            )
        )

        print(f"✅ Created Qdrant collection: {collection_name} (dimension={dimension})")

    def delete_collection(self, name: str):
        """Delete a Qdrant collection"""
        collection_name = self.db_config.get('collection_name', 'benchmark_test')

        try:
            self.client.delete_collection(collection_name)
        except:
            pass

    def insert_vectors(
        self,
        collection: str,
        vectors: np.ndarray,
        metadata: Optional[List[Dict[str, Any]]] = None,
        batch_size: int = 100
    ):
        """Insert vectors in batches"""
        from qdrant_client.models import PointStruct

        collection_name = self.db_config.get('collection_name', 'benchmark_test')
        num_vectors = len(vectors)
        num_batches = (num_vectors + batch_size - 1) // batch_size

        with tqdm(total=num_vectors, desc=f"Inserting to {self.name}") as pbar:
            for i in range(num_batches):
                start_idx = i * batch_size
                end_idx = min(start_idx + batch_size, num_vectors)

                batch_vectors = vectors[start_idx:end_idx]
                batch_metadata = metadata[start_idx:end_idx] if metadata else None

                # Prepare batch for Qdrant
                points = []
                for j, vector in enumerate(batch_vectors):
                    point_id = start_idx + j
                    meta = batch_metadata[j] if batch_metadata else {}

                    points.append(PointStruct(
                        id=point_id,
                        vector=vector.tolist(),
                        payload=meta
                    ))

                # Upsert batch
                self.client.upsert(
                    collection_name=collection_name,
                    points=points
                )

                pbar.update(len(batch_vectors))

    def search_vectors(
        self,
        collection: str,
        query_vectors: np.ndarray,
        top_k: int = 10
    ) -> List[List[Dict[str, Any]]]:
        """Search for similar vectors"""
        collection_name = self.db_config.get('collection_name', 'benchmark_test')
        results = []

        for query in query_vectors:
            result = self.client.search(
                collection_name=collection_name,
                query_vector=query.tolist(),
                limit=top_k
            )
            results.append(result)

        return results


if __name__ == "__main__":
    import yaml
    from pathlib import Path
    from data_generator import DataGenerator

    # Load config
    config_path = Path(__file__).parent / "config.yaml"
    with open(config_path) as f:
        config = yaml.safe_load(f)

    # Create benchmark instance
    benchmark = QdrantBenchmark(config['databases'])

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
        benchmark.delete_collection("test")

    finally:
        benchmark.disconnect()
