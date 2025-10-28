#!/usr/bin/env python3
"""
Run comprehensive competitive benchmarks.
"""

import yaml
import argparse
from pathlib import Path
from typing import List, Dict, Any
import json
from datetime import datetime

from data_generator import DataGenerator
from benchmark_base import BenchmarkRunner
from benchmark_dvecdb import DVecDBBenchmark
from benchmark_pinecone import PineconeBenchmark
from benchmark_qdrant import QdrantBenchmark


def run_benchmarks(
    databases: List[str],
    datasets: List[str],
    config: Dict[str, Any]
):
    """Run comprehensive benchmarks"""

    results_dir = Path(config['output']['results_dir'])
    results_dir.mkdir(parents=True, exist_ok=True)

    runner = BenchmarkRunner(config)

    # Initialize database clients
    db_clients = {}

    if 'dvecdb' in databases:
        db_clients['dvecdb'] = DVecDBBenchmark(config['databases'])

    if 'pinecone' in databases:
        db_clients['pinecone'] = PineconeBenchmark(config['databases'])

    if 'qdrant' in databases:
        db_clients['qdrant'] = QdrantBenchmark(config['databases'])

    # Connect all clients
    print("\n" + "="*80)
    print("CONNECTING TO DATABASES")
    print("="*80)

    for name, client in db_clients.items():
        try:
            client.connect()
        except Exception as e:
            print(f"❌ Failed to connect to {name}: {e}")
            print(f"   Skipping {name}")
            del db_clients[name]

    if not db_clients:
        print("❌ No databases available for benchmarking")
        return

    # Run benchmarks for each dataset
    for dataset_name in datasets:
        dataset_config = config['datasets'][dataset_name]

        print("\n" + "="*80)
        print(f"DATASET: {dataset_config['name']}")
        print(f"  Vectors: {dataset_config['num_vectors']:,}")
        print(f"  Dimension: {dataset_config['dimension']}")
        print("="*80)

        # Load or generate data
        data_dir = Path(__file__).parent / "datasets"
        data_dir.mkdir(parents=True, exist_ok=True)

        try:
            # Try to load existing data
            vectors, metadata = DataGenerator.load_dataset(data_dir, dataset_name)
            queries = DataGenerator.load_dataset(data_dir, f"{dataset_name}_queries")[0]
            print(f"✅ Loaded existing dataset: {dataset_name}")

        except FileNotFoundError:
            # Generate new data
            print(f"Generating dataset: {dataset_name}...")

            generator = DataGenerator(dimension=dataset_config['dimension'])

            vectors, _ = generator.generate_clustered_vectors(
                count=dataset_config['num_vectors']
            )
            metadata = generator.generate_metadata(len(vectors))

            generator.save_dataset(vectors, metadata, data_dir, dataset_name)

            # Generate queries
            queries, _ = generator.generate_clustered_vectors(
                count=dataset_config['query_count']
            )

        print(f"\nDataset ready: {len(vectors):,} vectors, {len(queries):,} queries\n")

        # Benchmark each database
        for db_name, db_client in db_clients.items():
            print(f"\n{'─'*80}")
            print(f"BENCHMARKING: {db_name.upper()}")
            print(f"{'─'*80}")

            try:
                # INSERT BENCHMARK
                print(f"\n1. INSERT BENCHMARK")
                print(f"   Inserting {len(vectors):,} vectors...")

                for batch_size in config['benchmarks']['insert']['batch_sizes']:
                    print(f"\n   Batch size: {batch_size}")

                    result = db_client.benchmark_insert(
                        dataset_name=dataset_name,
                        vectors=vectors,
                        metadata=metadata,
                        batch_size=batch_size
                    )

                    runner.add_result(result)

                    print(f"   ✅ Throughput: {result.throughput:,.0f} vectors/sec")
                    print(f"      Duration: {result.duration_seconds:.2f}s")
                    print(f"      Memory: {result.memory_mb:.2f} MB")

                # SEARCH BENCHMARK
                print(f"\n2. SEARCH BENCHMARK")
                print(f"   Searching with {len(queries):,} queries...")

                for top_k in config['benchmarks']['search']['top_k_values']:
                    print(f"\n   Top-K: {top_k}")

                    result = db_client.benchmark_search(
                        dataset_name=dataset_name,
                        query_vectors=queries,
                        top_k=top_k,
                        warmup=config['benchmarks']['search']['warmup']
                    )

                    runner.add_result(result)

                    print(f"   ✅ Throughput: {result.throughput:,.0f} queries/sec")
                    print(f"      P50 Latency: {result.latency_p50:.2f} ms")
                    print(f"      P95 Latency: {result.latency_p95:.2f} ms")
                    print(f"      P99 Latency: {result.latency_p99:.2f} ms")

                # CONCURRENT SEARCH BENCHMARK
                print(f"\n3. CONCURRENT SEARCH BENCHMARK")

                for concurrent in config['benchmarks']['search']['concurrent_queries']:
                    print(f"\n   Concurrent clients: {concurrent}")

                    result = db_client.benchmark_concurrent_search(
                        dataset_name=dataset_name,
                        query_vectors=queries,
                        top_k=10,
                        concurrent=concurrent
                    )

                    runner.add_result(result)

                    print(f"   ✅ Throughput: {result.throughput:,.0f} queries/sec")
                    print(f"      P50 Latency: {result.latency_p50:.2f} ms")
                    print(f"      P99 Latency: {result.latency_p99:.2f} ms")

                # Cleanup
                db_client.delete_collection(dataset_name)

            except Exception as e:
                print(f"\n❌ Benchmark failed for {db_name}: {e}")
                import traceback
                traceback.print_exc()

    # Disconnect all clients
    print("\n" + "="*80)
    print("DISCONNECTING FROM DATABASES")
    print("="*80)

    for name, client in db_clients.items():
        try:
            client.disconnect()
            print(f"✅ Disconnected from {name}")
        except:
            pass

    # Export results
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    output_file = results_dir / f"benchmark_results_{timestamp}.json"

    runner.export_results(str(output_file))

    print("\n" + "="*80)
    print("BENCHMARK COMPLETE!")
    print("="*80)
    print(f"\nResults saved to: {output_file}")
    print(f"Total benchmarks run: {len(runner.results)}")

    return runner.results


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Run competitive vector database benchmarks")

    parser.add_argument(
        '--databases',
        nargs='+',
        choices=['dvecdb', 'pinecone', 'qdrant'],
        default=['dvecdb', 'qdrant'],
        help='Databases to benchmark'
    )

    parser.add_argument(
        '--datasets',
        nargs='+',
        choices=['small', 'medium', 'large', 'realistic'],
        default=['small'],
        help='Datasets to use for benchmarking'
    )

    parser.add_argument(
        '--config',
        type=str,
        default='config.yaml',
        help='Path to configuration file'
    )

    args = parser.parse_args()

    # Load config
    config_path = Path(args.config)
    with open(config_path) as f:
        config = yaml.safe_load(f)

    # Run benchmarks
    results = run_benchmarks(
        databases=args.databases,
        datasets=args.datasets,
        config=config
    )
