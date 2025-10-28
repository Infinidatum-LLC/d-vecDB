#!/usr/bin/env python3
"""
Test data generator for competitive benchmarking.
Generates realistic vector datasets for benchmarking.
"""

import numpy as np
from typing import List, Tuple, Dict, Any
import json
from pathlib import Path
from tqdm import tqdm


class DataGenerator:
    """Generate test data for benchmarking"""

    def __init__(self, dimension: int, seed: int = 42):
        self.dimension = dimension
        self.rng = np.random.RandomState(seed)

    def generate_random_vectors(self, count: int) -> np.ndarray:
        """Generate random normalized vectors"""
        vectors = self.rng.randn(count, self.dimension).astype(np.float32)

        # Normalize to unit length (cosine similarity)
        norms = np.linalg.norm(vectors, axis=1, keepdims=True)
        vectors = vectors / (norms + 1e-8)

        return vectors

    def generate_clustered_vectors(
        self,
        count: int,
        num_clusters: int = 10
    ) -> Tuple[np.ndarray, np.ndarray]:
        """Generate clustered vectors (more realistic)"""
        from sklearn.datasets import make_blobs

        vectors, labels = make_blobs(
            n_samples=count,
            n_features=self.dimension,
            centers=num_clusters,
            random_state=self.rng.randint(0, 10000)
        )

        vectors = vectors.astype(np.float32)

        # Normalize
        norms = np.linalg.norm(vectors, axis=1, keepdims=True)
        vectors = vectors / (norms + 1e-8)

        return vectors, labels

    def generate_metadata(self, count: int) -> List[Dict[str, Any]]:
        """Generate realistic metadata"""
        categories = ["tech", "science", "business", "sports", "entertainment"]
        sources = ["web", "api", "upload", "crawl"]
        priorities = ["low", "medium", "high", "critical"]

        metadata = []
        for i in range(count):
            meta = {
                "id": f"doc_{i}",
                "category": self.rng.choice(categories),
                "source": self.rng.choice(sources),
                "priority": self.rng.choice(priorities),
                "timestamp": 1600000000 + i * 100,
                "score": float(self.rng.uniform(0, 1)),
                "tags": [
                    self.rng.choice(["tag1", "tag2", "tag3", "tag4", "tag5"])
                    for _ in range(self.rng.randint(1, 4))
                ]
            }
            metadata.append(meta)

        return metadata

    def save_dataset(
        self,
        vectors: np.ndarray,
        metadata: List[Dict[str, Any]],
        path: Path,
        name: str
    ):
        """Save dataset to disk"""
        path.mkdir(parents=True, exist_ok=True)

        # Save vectors (binary format for efficiency)
        vectors_file = path / f"{name}_vectors.npy"
        np.save(vectors_file, vectors)

        # Save metadata (JSON)
        metadata_file = path / f"{name}_metadata.json"
        with open(metadata_file, 'w') as f:
            json.dump(metadata, f)

        # Save info
        info = {
            "name": name,
            "count": len(vectors),
            "dimension": self.dimension,
            "vectors_file": str(vectors_file),
            "metadata_file": str(metadata_file)
        }

        info_file = path / f"{name}_info.json"
        with open(info_file, 'w') as f:
            json.dump(info, f, indent=2)

        print(f"Dataset saved: {name}")
        print(f"  Vectors: {len(vectors):,}")
        print(f"  Dimension: {self.dimension}")
        print(f"  Size: {vectors.nbytes / 1024 / 1024:.2f} MB")

    @staticmethod
    def load_dataset(path: Path, name: str) -> Tuple[np.ndarray, List[Dict[str, Any]]]:
        """Load dataset from disk"""
        vectors_file = path / f"{name}_vectors.npy"
        metadata_file = path / f"{name}_metadata.json"

        vectors = np.load(vectors_file)

        with open(metadata_file, 'r') as f:
            metadata = json.load(f)

        return vectors, metadata


def generate_all_datasets(config: Dict[str, Any], output_dir: Path):
    """Generate all benchmark datasets"""
    print("Generating benchmark datasets...")

    for dataset_name, dataset_config in config['datasets'].items():
        print(f"\nGenerating dataset: {dataset_config['name']}")

        generator = DataGenerator(
            dimension=dataset_config['dimension'],
            seed=42
        )

        # Generate vectors
        print("  Generating vectors...")
        vectors, _ = generator.generate_clustered_vectors(
            count=dataset_config['num_vectors'],
            num_clusters=max(10, dataset_config['num_vectors'] // 1000)
        )

        # Generate metadata
        print("  Generating metadata...")
        metadata = generator.generate_metadata(len(vectors))

        # Save dataset
        generator.save_dataset(
            vectors=vectors,
            metadata=metadata,
            path=output_dir / "datasets",
            name=dataset_name
        )

        # Generate query vectors (separate from dataset)
        print("  Generating query vectors...")
        query_vectors, _ = generator.generate_clustered_vectors(
            count=dataset_config['query_count'],
            num_clusters=max(5, dataset_config['query_count'] // 100)
        )

        query_file = output_dir / "datasets" / f"{dataset_name}_queries.npy"
        np.save(query_file, query_vectors)

    print("\n✅ All datasets generated successfully!")


if __name__ == "__main__":
    import yaml
    from pathlib import Path

    # Load config
    config_path = Path(__file__).parent / "config.yaml"
    with open(config_path) as f:
        config = yaml.safe_load(f)

    # Generate datasets
    output_dir = Path(__file__).parent
    generate_all_datasets(config, output_dir)
