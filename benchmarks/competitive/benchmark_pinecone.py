#!/usr/bin/env python3
"""
Pinecone benchmark client.
"""

import os
import time
import numpy as np
from typing import Dict, List, Any, Optional
from benchmark_base import VectorDBBenchmark
from tqdm import tqdm


class PineconeBenchmark(VectorDBBenchmark):
    """Pinecone benchmark implementation"""

    def __init__(self, config: Dict[str, Any]):
        super().__init__("Pinecone", config)
        self.client = None
        self.index = None
        self.db_config = config.get('pinecone', {})

    def connect(self):
        """Connect to Pinecone"""
        try:
            from pinecone import Pinecone, ServerlessSpec
        except ImportError:
            print("❌ Pinecone client not installed. Install with: pip install pinecone-client")
            raise

        api_key = os.getenv('PINECONE_API_KEY')
        if not api_key:
            raise ValueError("PINECONE_API_KEY environment variable not set")

        self.client = Pinecone(api_key=api_key)

        print(f"✅ Connected to Pinecone")

    def disconnect(self):
        """Disconnect from Pinecone"""
        self.client = None
        self.index = None

    def create_collection(self, name: str, dimension: int):
        """Create a Pinecone index"""
        index_name = self.db_config.get('index_name', 'benchmark-test')

        # Delete if exists
        try:
            self.client.delete_index(index_name)
            time.sleep(5)  # Wait for deletion
        except:
            pass

        # Create new index
        from pinecone import ServerlessSpec

        self.client.create_index(
            name=index_name,
            dimension=dimension,
            metric=self.db_config.get('metric', 'cosine'),
            spec=ServerlessSpec(
                cloud='aws',
                region=self.db_config.get('environment', 'us-west-2')
            )
        )

        # Wait for index to be ready
        while not self.client.describe_index(index_name).status['ready']:
            time.sleep(1)

        self.index = self.client.Index(index_name)

        print(f"✅ Created Pinecone index: {index_name} (dimension={dimension})")

    def delete_collection(self, name: str):
        """Delete a Pinecone index"""
        index_name = self.db_config.get('index_name', 'benchmark-test')

        try:
            self.client.delete_index(index_name)
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
        num_vectors = len(vectors)
        num_batches = (num_vectors + batch_size - 1) // batch_size

        with tqdm(total=num_vectors, desc=f"Inserting to {self.name}") as pbar:
            for i in range(num_batches):
                start_idx = i * batch_size
                end_idx = min(start_idx + batch_size, num_vectors)

                batch_vectors = vectors[start_idx:end_idx]
                batch_metadata = metadata[start_idx:end_idx] if metadata else None

                # Prepare batch for Pinecone
                upsert_data = []
                for j, vector in enumerate(batch_vectors):
                    vector_id = f"vec_{start_idx + j}"
                    meta = batch_metadata[j] if batch_metadata else {}

                    upsert_data.append({
                        "id": vector_id,
                        "values": vector.tolist(),
                        "metadata": meta
                    })

                # Upsert batch
                self.index.upsert(vectors=upsert_data)

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
            result = self.index.query(
                vector=query.tolist(),
                top_k=top_k,
                include_metadata=True
            )
            results.append(result.get('matches', []))

        return results


if __name__ == "__main__":
    import yaml
    from pathlib import Path
    from data_generator import DataGenerator

    # Check for API key
    if not os.getenv('PINECONE_API_KEY'):
        print("❌ PINECONE_API_KEY environment variable not set")
        print("Set it with: export PINECONE_API_KEY=your_api_key")
        exit(1)

    # Load config
    config_path = Path(__file__).parent / "config.yaml"
    with open(config_path) as f:
        config = yaml.safe_load(f)

    # Create benchmark instance
    benchmark = PineconeBenchmark(config['databases'])

    try:
        # Connect
        benchmark.connect()

        # Generate test data (small test)
        print("\nGenerating test data...")
        generator = DataGenerator(dimension=128)
        vectors, _ = generator.generate_clustered_vectors(count=100)
        metadata = generator.generate_metadata(len(vectors))
        query_vectors, _ = generator.generate_clustered_vectors(count=10)

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

        # Test search
        print("\nTesting search...")
        result = benchmark.benchmark_search(
            dataset_name="test",
            query_vectors=query_vectors,
            top_k=10,
            warmup=5
        )

        print(f"\nSearch Results:")
        print(f"  Throughput: {result.throughput:.2f} queries/sec")
        print(f"  P50 Latency: {result.latency_p50:.2f} ms")
        print(f"  P99 Latency: {result.latency_p99:.2f} ms")

    finally:
        # Cleanup
        try:
            benchmark.delete_collection("test")
        except:
            pass
        benchmark.disconnect()
