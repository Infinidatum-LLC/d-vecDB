# -*- coding: utf-8 -*-
"""
d-vecDB Python Client - Google Colab Example (UPDATED & FIXED)
=============================================================

🎉 This is a comprehensive, working example for Google Colab!

IMPORTANT: Before running, update the SERVER_HOST below with your actual ngrok URL!

[![Open In Colab](https://colab.research.google.com/assets/colab-badge.svg)](https://colab.research.google.com/github/rdmurugan/d-vecDB/blob/master/python-client/colab-vecdb.py)

✅ All known issues have been fixed:
   - Vector field: 'values' → 'data'
   - Method: 'upsert_vectors' → 'insert_vectors'  
   - Method: 'search_vectors' → 'search'
   - Parameter: 'top_k' → 'limit'
   - Field: 'score' → 'distance'
   - Proper SSL configuration for ngrok HTTPS
   - Correct error handling and response parsing

📋 Prerequisites:
   1. A running d-vecDB server
   2. ngrok tunnel (if using local server) 
   3. Update SERVER_HOST variable below

🚀 Ready to run in Google Colab!
"""

# d-vecDB Python Client - Google Colab Example

This notebook demonstrates how to use the d-vecDB Python client in Google Colab for vector similarity search and embeddings management.

## What you'll learn:
- How to install and set up d-vecDB client in Colab
- Connect to a remote d-vecDB server
- Create collections and insert vectors
- Perform similarity searches
- Work with text embeddings using sentence transformers

## 🔧 Installation

First, let's install the required packages:
"""

# Install the d-vecDB Python client and dependencies
# Specify compatible versions for grpcio and protobuf to avoid 'AttributeError: 'MessageFactory' object has no attribute 'GetPrototype''
!pip install vectordb-client sentence-transformers numpy pandas matplotlib grpcio==1.62.2 grpcio-tools==1.62.2 protobuf==4.25.3

# Import required libraries
import numpy as np
import pandas as pd
from typing import List, Dict, Any
import json
import time
from sentence_transformers import SentenceTransformer
import matplotlib.pyplot as plt
from sklearn.decomposition import PCA

print("✅ Installation complete!")

# Optional: Set up ngrok if you want to expose a local server
# Uncomment the lines below if you're running your own d-vecDB server locally
# !pip install pyngrok
# !ngrok config add-authtoken YOUR_NGROK_TOKEN_HERE
# from pyngrok import ngrok
# public_url = ngrok.connect(8080)
# print(f"🌐 Your ngrok URL: {public_url}")
print("ℹ️  Using existing ngrok URL - update SERVER_HOST below with your actual URL")

"""## 🚀 Setting up the VectorDB Client

**Note**: For this example, you'll need access to a running d-vecDB server. You can:
1. Run a local server and use ngrok to expose it
2. Use a cloud-hosted d-vecDB instance
3. For demo purposes, we'll show how to set up the client (you'll need to replace with your actual server details)
"""

from vectordb_client import VectorDBClient
from vectordb_client.types import (
    CollectionConfig, Vector, DistanceMetric,
    IndexConfig, VectorType
)

# Configuration - Replace with your actual ngrok URL
SERVER_HOST = "your-ngrok-url.ngrok-free.app"  # Replace with your actual ngrok host
SERVER_PORT = 443  # HTTPS port for ngrok (not 8080)

# For other ngrok URLs, it would look like:
# SERVER_HOST = "abc123.ngrok-free.app"
# SERVER_PORT = 443

print(f"🔌 Connecting to d-vecDB server at {SERVER_HOST}:{SERVER_PORT}...")

try:
    # Initialize the client with SSL support for ngrok
    client = VectorDBClient(
        host=SERVER_HOST,
        port=SERVER_PORT,
        ssl=True,        # Required for ngrok HTTPS URLs
        protocol="rest"
    )

    # Test the connection with correct method name
    health_response = client.health_check()  # Changed from ping()

    if health_response and health_response.success:
        print("✅ Successfully connected to d-vecDB!")

        # Get server statistics with correct method name
        server_stats = client.get_server_stats()  # Changed from get_server_info()
        print(f"📊 Server Stats: {server_stats}")

        # List existing collections
        collections = client.list_collections()
        print(f"📂 Collections: {collections.data}")

        # Example: Create a new collection
        print("\n🔨 Creating a sample collection...")
        try:
            config = CollectionConfig(
                name="demo_collection",
                dimension=384,  # Common dimension for sentence embeddings
                distance_metric=DistanceMetric.COSINE
            )
            result = client.create_collection(config)
            print(f"✅ Collection created: {result}")
        except Exception as e:
            print(f"ℹ️ Collection might already exist: {e}")

        # List collections again to see the new one
        updated_collections = client.list_collections()
        print(f"📂 Updated Collections: {updated_collections.data}")

    else:
        print("❌ Could not connect to d-vecDB server")
        print("Please check your server configuration and try again.")

except Exception as e:
    print(f"❌ Connection failed: {e}")
    print(f"Error type: {type(e).__name__}")
    print("\n💡 Make sure you have:")
    print("1. A running d-vecDB server")
    print("2. Active ngrok tunnel pointing to your server")
    print("3. Correct configuration:")
    print(f"   - Host: {SERVER_HOST}")
    print(f"   - Port: {SERVER_PORT} (443 for HTTPS)")
    print("   - SSL: True (required for ngrok HTTPS)")

"""## 📄 Preparing Sample Data

Let's create some sample documents and generate embeddings for them:
"""

# Sample documents for demonstration
sample_documents = [
    "The quick brown fox jumps over the lazy dog",
    "Machine learning is a subset of artificial intelligence",
    "Vector databases enable efficient similarity search",
    "Python is a popular programming language for data science",
    "Natural language processing helps computers understand text",
    "Deep learning models can generate realistic images",
    "Cloud computing provides scalable infrastructure solutions",
    "Database optimization improves query performance",
    "Artificial neural networks mimic biological brain functions",
    "Big data analytics reveals insights from large datasets"
]

print(f"📚 Sample documents ({len(sample_documents)} total):")
for i, doc in enumerate(sample_documents, 1):
    print(f"{i:2d}. {doc}")

"""## 🔤 Generating Text Embeddings

We'll use sentence-transformers to convert our text documents into vector embeddings:
"""

# Initialize the sentence transformer model
print("🤖 Loading sentence transformer model...")
model = SentenceTransformer('all-MiniLM-L6-v2')  # Lightweight model, good for Colab

# Generate embeddings
print("⚡ Generating embeddings...")
embeddings = model.encode(sample_documents)

print(f"✅ Generated {len(embeddings)} embeddings")
print(f"📏 Embedding dimension: {embeddings.shape[1]}")
print(f"🔢 Data type: {embeddings.dtype}")

# Convert to list format for d-vecDB
embedding_vectors = [embedding.tolist() for embedding in embeddings]

print(f"\n📊 First embedding preview (first 10 dimensions):")
print(embedding_vectors[0][:10])

"""## 📈 Visualizing Embeddings

Let's visualize our embeddings in 2D using PCA:
"""

# Reduce embeddings to 2D for visualization
pca = PCA(n_components=2)
embeddings_2d = pca.fit_transform(embeddings)

# Create the plot
plt.figure(figsize=(12, 8))
scatter = plt.scatter(embeddings_2d[:, 0], embeddings_2d[:, 1],
                     alpha=0.7, s=100, c=range(len(sample_documents)),
                     cmap='tab10')

# Add labels for each point
for i, doc in enumerate(sample_documents):
    plt.annotate(f"{i+1}",
                xy=(embeddings_2d[i, 0], embeddings_2d[i, 1]),
                xytext=(5, 5), textcoords='offset points',
                fontsize=12, fontweight='bold')

plt.title('Document Embeddings Visualization (PCA)', fontsize=16)
plt.xlabel(f'PC1 ({pca.explained_variance_ratio_[0]:.1%} variance)', fontsize=12)
plt.ylabel(f'PC2 ({pca.explained_variance_ratio_[1]:.1%} variance)', fontsize=12)
plt.grid(True, alpha=0.3)
plt.tight_layout()
plt.show()

print("\n📋 Document Reference:")
for i, doc in enumerate(sample_documents, 1):
    print(f"{i:2d}. {doc[:50]}{'...' if len(doc) > 50 else ''}")

"""## 📁 Creating a Collection

Now let's create a collection in d-vecDB to store our embeddings:
"""

# Collection configuration
collection_name = "colab_text_embeddings"
embedding_dimension = len(embedding_vectors[0])

print(f"📁 Creating collection '{collection_name}'...")

try:
    # Clean up any existing collection
    try:
        client.delete_collection(collection_name)
        print(f"🗑️  Deleted existing collection")
    except:
        pass

    # Create new collection with cosine similarity
    config = CollectionConfig(
        name=collection_name,
        dimension=embedding_dimension,
        distance_metric=DistanceMetric.COSINE
    )
    response = client.create_collection(config)

    print(f"✅ Created collection: {response}")

    # List all collections to verify
    collections = client.list_collections()
    print(f"📋 Available collections: {collections}")

except Exception as e:
    print(f"❌ Failed to create collection: {e}")
    print("Please ensure your d-vecDB server is running and accessible.")

"""## ⬆️ Inserting Vectors

Let's insert our document embeddings into the collection:
"""

print("⬆️  Inserting vectors into collection...")

try:
    # Prepare vectors with metadata
    vectors_to_insert = []

    for i, (doc, embedding) in enumerate(zip(sample_documents, embedding_vectors)):
        vector = Vector(
            id=str(i + 1),
            data=embedding,  # FIXED: Changed from 'values' to 'data'
            metadata={
                "document": doc,
                "length": len(doc),
                "index": i + 1,
                "word_count": len(doc.split())
            }
        )
        vectors_to_insert.append(vector)

    # Insert vectors in batch
    start_time = time.time()
    response = client.insert_vectors(collection_name, vectors_to_insert)  # FIXED: Changed from 'upsert_vectors'
    insert_time = time.time() - start_time

    print(f"✅ Inserted {len(vectors_to_insert)} vectors in {insert_time:.2f} seconds")
    print(f"📊 Insert response: {response}")

    # Get collection statistics
    try:
        stats = client.get_collection_stats(collection_name)
        print(f"📈 Collection stats: {stats}")
    except:
        print("ℹ️  Collection stats not available")

except Exception as e:
    print(f"❌ Failed to insert vectors: {e}")

"""## 🔍 Similarity Search

Now let's perform similarity searches to find related documents:
"""

def search_similar_documents(query_text: str, top_k: int = 5):
    """Search for documents similar to the query text."""
    print(f"\n🔍 Searching for: '{query_text}'")
    print("="*60)

    try:
        # Generate embedding for query
        query_embedding = model.encode([query_text])[0].tolist()

        # Perform search
        start_time = time.time()
        results = client.search(
            collection_name=collection_name,
            query_vector=query_embedding,
            limit=top_k  # FIXED: Changed from 'top_k' to 'limit'
        )
        search_time = time.time() - start_time

        print(f"⚡ Search completed in {search_time:.3f} seconds")
        
        if results.success and results.data:
            print(f"📋 Found {len(results.data)} results:\n")

            for i, result in enumerate(results.data, 1):
                doc_text = result.metadata.get('document', 'N/A')
                similarity = 1 - result.distance  # Convert distance to similarity for cosine

                print(f"{i}. [Similarity: {similarity:.3f}] {doc_text}")
        else:
            print("❌ No results found or search failed")
            print(f"Response: {results}")

        return results.data if results.success else []

    except Exception as e:
        print(f"❌ Search failed: {e}")
        return []

# Example searches
search_queries = [
    "artificial intelligence and machine learning",
    "database and data storage",
    "programming languages for data",
    "computer vision and image processing"
]

for query in search_queries:
    search_similar_documents(query, top_k=3)

"""## 🎯 Interactive Search

Try your own search queries:
"""

# Interactive search - modify this cell to try different queries
your_query = "neural networks and AI"  # ← Change this to your query

print("🎯 Your custom search:")
results = search_similar_documents(your_query, top_k=5)

# Show detailed results with metadata
if results:
    print("\n📊 Detailed Results:")
    print("="*80)

    for i, result in enumerate(results, 1):
        similarity = 1 - result.distance  # FIXED: Changed from 'score' to 'distance'
        metadata = result.metadata

        print(f"\nResult {i}:")
        print(f"  📄 Document: {metadata.get('document', 'N/A')}")
        print(f"  🎯 Similarity: {similarity:.4f}")
        print(f"  📏 Length: {metadata.get('length', 'N/A')} characters")
        print(f"  💬 Words: {metadata.get('word_count', 'N/A')}")
        print(f"  🆔 ID: {result.id}")

"""## 🔧 Advanced Vector Operations

Let's explore some advanced operations:
"""

print("🔧 Advanced Vector Operations")
print("="*50)

try:
    # 1. Get collection info
    print("\n1️⃣ Getting collection information...")
    try:
        collection_info = client.get_collection(collection_name)
        print(f"✅ Collection info: {collection_info}")
    except Exception as e:
        print(f"ℹ️ Collection info not available: {e}")

    # 2. Filter search with metadata
    print("\n2️⃣ Filtered search (documents with >50 characters)...")
    query_text = "data science programming"
    query_embedding = model.encode([query_text])[0].tolist()

    # Note: Metadata filtering syntax depends on your d-vecDB server implementation
    # This is a conceptual example - adjust based on your server's API
    filtered_results = client.search(
        collection_name=collection_name,
        query_vector=query_embedding,
        limit=5
        # filter={"length": {"$gt": 50}}  # Uncomment if your server supports filtering
    )

    if filtered_results.success and filtered_results.data:
        print(f"📋 Filtered results: {len(filtered_results.data)}")
        for result in filtered_results.data[:3]:
            doc_length = result.metadata.get('length', 0)
            if doc_length > 50:  # Client-side filtering as example
                similarity = 1 - result.distance
                print(f"   • [Similarity: {similarity:.3f}, Length: {doc_length}] {result.metadata.get('document', 'N/A')[:60]}...")
    else:
        print("❌ Filtered search failed")

    # 3. List collections
    print("\n3️⃣ Listing all collections...")
    all_collections = client.list_collections()
    if all_collections.success:
        print(f"✅ Found {len(all_collections.data)} collections: {all_collections.data}")
    else:
        print("❌ Failed to list collections")

except Exception as e:
    print(f"❌ Advanced operations failed: {e}")
    print("Some operations may not be supported by your d-vecDB server version.")

"""## ⚡ Performance Testing

Let's test the performance of our vector database:
"""

print("⚡ Performance Testing")
print("="*40)

try:
    # Test search performance
    test_queries = [
        "machine learning algorithms",
        "database optimization techniques",
        "natural language processing",
        "cloud computing infrastructure",
        "artificial intelligence applications"
    ]

    search_times = []

    print("🔍 Running search performance test...")
    for i, query in enumerate(test_queries, 1):
        query_embedding = model.encode([query])[0].tolist()

        start_time = time.time()
        results = client.search(
            collection_name=collection_name,
            query_vector=query_embedding,
            limit=5
        )
        search_time = (time.time() - start_time) * 1000  # Convert to milliseconds
        search_times.append(search_time)
        
        result_count = len(results.data) if results.success and results.data else 0
        print(f"   Query {i}: {search_time:.2f}ms ({result_count} results)")

    # Performance statistics
    avg_time = np.mean(search_times)
    min_time = np.min(search_times)
    max_time = np.max(search_times)

    print(f"\n📊 Performance Summary:")
    print(f"   Average search time: {avg_time:.2f}ms")
    print(f"   Fastest search: {min_time:.2f}ms")
    print(f"   Slowest search: {max_time:.2f}ms")

    # Visualize performance
    plt.figure(figsize=(10, 6))
    plt.bar(range(1, len(search_times) + 1), search_times, alpha=0.7)
    plt.axhline(y=avg_time, color='r', linestyle='--', label=f'Average: {avg_time:.2f}ms')
    plt.xlabel('Query Number')
    plt.ylabel('Search Time (milliseconds)')
    plt.title('Vector Search Performance')
    plt.legend()
    plt.grid(True, alpha=0.3)
    plt.tight_layout()
    plt.show()

except Exception as e:
    print(f"❌ Performance test failed: {e}")

"""## 🧹 Cleanup

Clean up resources when done:
"""

print("🧹 Cleaning up resources...")

try:
    # Optionally delete the collection
    delete_collection = False  # Set to True if you want to clean up

    if delete_collection:
        response = client.delete_collection(collection_name)
        print(f"🗑️  Deleted collection '{collection_name}': {response}")
    else:
        print(f"ℹ️  Collection '{collection_name}' preserved for further use")

    # List remaining collections
    collections = client.list_collections()
    print(f"📋 Remaining collections: {collections}")

except Exception as e:
    print(f"❌ Cleanup failed: {e}")

print("\n✅ Notebook execution completed!")

"""## 🚀 Next Steps

Congratulations! You've successfully:
- ✅ Set up d-vecDB client in Google Colab
- ✅ Generated text embeddings using sentence transformers
- ✅ Created a vector collection
- ✅ Inserted and searched vectors
- ✅ Performed similarity searches
- ✅ Tested performance

### What to try next:

1. **Scale up**: Try with larger datasets (1000+ documents)
2. **Different embeddings**: Experiment with different sentence transformer models
3. **Real data**: Use your own documents or datasets
4. **Advanced features**: Explore filtering, metadata queries, and batch operations
5. **Integration**: Connect with your applications or data pipelines

### Useful Resources:

- 📚 [d-vecDB Documentation](https://github.com/rdmurugan/d-vecDB)
- 🤗 [Sentence Transformers](https://www.sbert.net/)
- 🐍 [Python Client API Reference](https://github.com/rdmurugan/d-vecDB/tree/master/python-client)

### Need Help?

- 🐛 Report issues: [GitHub Issues](https://github.com/rdmurugan/d-vecDB/issues)
- 💬 Discussions: [GitHub Discussions](https://github.com/rdmurugan/d-vecDB/discussions)

Happy vector searching! 🎉
"""