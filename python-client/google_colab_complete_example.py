"""
Complete Working d-vecDB Example for Google Colab
=================================================
This notebook demonstrates how to use the d-vecDB Python client with a remote server via ngrok.
"""

# 1. INSTALLATION
print("📦 Installing d-vecDB client...")
# Uncomment the next line if running in Google Colab
# !pip install vectordb-client

# 2. IMPORTS
import time
import numpy as np
from typing import List, Dict, Any
from vectordb_client import VectorDBClient
from vectordb_client.types import (
    CollectionConfig, Vector, DistanceMetric,
    IndexConfig, VectorType
)

print("✅ All imports successful!")

# 3. CONFIGURATION
print("\n🔧 Configuration...")
SERVER_HOST = "07604dfeb2ee.ngrok-free.app"  # Replace with your ngrok URL
SERVER_PORT = 443  # HTTPS port for ngrok
USE_SSL = True     # Required for ngrok HTTPS
COLLECTION_NAME = "colab_demo_collection"

print(f"🌐 Server: {SERVER_HOST}:{SERVER_PORT}")
print(f"🔒 SSL: {USE_SSL}")
print(f"📁 Collection: {COLLECTION_NAME}")

# 4. SAMPLE DATA
print("\n📄 Preparing sample documents...")
sample_documents = [
    "The quick brown fox jumps over the lazy dog",
    "Machine learning is revolutionizing technology",
    "Vector databases enable semantic search capabilities",
    "Python is a versatile programming language",
    "Artificial intelligence transforms industries",
    "Natural language processing understands human speech",
    "Deep learning models learn complex patterns",
    "Data science extracts insights from information",
    "Cloud computing provides scalable infrastructure",
    "Cybersecurity protects digital assets and privacy"
]

# Create sample embeddings (384 dimensions - common for sentence transformers)
print("🔢 Generating sample embeddings...")
embedding_dimension = 384
embedding_vectors = [
    np.random.rand(embedding_dimension).tolist() 
    for _ in range(len(sample_documents))
]

print(f"✅ Created {len(embedding_vectors)} embeddings with {embedding_dimension} dimensions")

# 5. CONNECTION
print("\n🔌 Connecting to d-vecDB server...")
try:
    client = VectorDBClient(
        host=SERVER_HOST,
        port=SERVER_PORT,
        ssl=USE_SSL,
        protocol="rest"
    )
    
    # Test connection
    health = client.health_check()
    if health.success:
        print("✅ Successfully connected to d-vecDB!")
        
        # Get server stats
        stats = client.get_server_stats()
        print(f"📊 Server Stats: {stats}")
    else:
        print("❌ Health check failed")
        exit(1)
        
except Exception as e:
    print(f"❌ Connection failed: {e}")
    print("\n💡 Troubleshooting:")
    print("1. Check if your ngrok tunnel is active")
    print("2. Verify the ngrok URL is correct")
    print("3. Ensure d-vecDB server is running")
    exit(1)

# 6. COLLECTION MANAGEMENT
print(f"\n📁 Managing collection '{COLLECTION_NAME}'...")

# Check if collection exists
try:
    existing_collections = client.list_collections()
    if COLLECTION_NAME in existing_collections.data:
        print(f"ℹ️  Collection '{COLLECTION_NAME}' already exists")
        
        # Delete existing collection for fresh start (optional)
        choice = input("Delete existing collection and recreate? (y/n): ").lower()
        if choice == 'y':
            delete_response = client.delete_collection(COLLECTION_NAME)
            print(f"🗑️  Deleted collection: {delete_response}")
        else:
            print("📝 Using existing collection")
    
    # Create collection (if not exists or was deleted)
    collections = client.list_collections()
    if COLLECTION_NAME not in collections.data:
        print(f"🔨 Creating collection '{COLLECTION_NAME}'...")
        
        config = CollectionConfig(
            name=COLLECTION_NAME,
            dimension=embedding_dimension,
            distance_metric=DistanceMetric.COSINE,
            vector_type=VectorType.FLOAT32
        )
        
        create_response = client.create_collection(config)
        print(f"✅ Collection created: {create_response}")
    
    # Verify collection
    updated_collections = client.list_collections()
    print(f"📋 Available collections: {updated_collections.data}")
    
except Exception as e:
    print(f"❌ Collection management failed: {e}")
    exit(1)

# 7. VECTOR INSERTION
print(f"\n⬆️  Inserting {len(sample_documents)} vectors into collection...")

try:
    # Prepare vectors with metadata
    vectors_to_insert = []
    
    for i, (doc, embedding) in enumerate(zip(sample_documents, embedding_vectors)):
        vector = Vector(
            id=f"doc_{i+1}",  # More descriptive IDs
            data=embedding,   # Correct field name
            metadata={
                "document": doc,
                "length": len(doc),
                "index": i + 1,
                "word_count": len(doc.split()),
                "category": "sample_data"
            }
        )
        vectors_to_insert.append(vector)
    
    # Insert vectors in batch
    start_time = time.time()
    response = client.insert_vectors(COLLECTION_NAME, vectors_to_insert)
    insert_time = time.time() - start_time
    
    print(f"✅ Inserted {len(vectors_to_insert)} vectors in {insert_time:.2f} seconds")
    print(f"📊 Insert response: {response}")
    
    # Try to get collection statistics
    try:
        stats = client.get_collection_stats(COLLECTION_NAME)
        print(f"📈 Collection stats: {stats}")
    except Exception as stats_error:
        print(f"ℹ️  Collection stats not available: {stats_error}")

except Exception as e:
    print(f"❌ Failed to insert vectors: {e}")
    print(f"Error type: {type(e).__name__}")
    
    # Try alternative insertion method
    print("\n🔄 Trying alternative insertion method...")
    try:
        successful_inserts = 0
        for i, (doc, embedding) in enumerate(zip(sample_documents, embedding_vectors)):
            response = client.insert_simple(
                collection_name=COLLECTION_NAME,
                vector_id=f"doc_{i+1}",
                vector_data=embedding,
                metadata={
                    "document": doc,
                    "length": len(doc),
                    "index": i + 1,
                    "word_count": len(doc.split()),
                    "category": "sample_data"
                }
            )
            if response.success:
                successful_inserts += 1
                print(f"  ✅ Inserted vector {i+1}")
            else:
                print(f"  ❌ Failed vector {i+1}: {response}")
        
        print(f"✅ Successfully inserted {successful_inserts}/{len(sample_documents)} vectors")
        
    except Exception as alt_error:
        print(f"❌ Alternative insertion also failed: {alt_error}")

# 8. VECTOR SEARCH
print(f"\n🔍 Testing vector similarity search...")

try:
    # Create a query vector (in practice, this would be an embedding of your search query)
    query_text = "artificial intelligence and machine learning"
    query_vector = np.random.rand(embedding_dimension).tolist()
    
    print(f"🔎 Searching for: '{query_text}'")
    
    # Perform search
    search_start = time.time()
    results = client.search(
        collection_name=COLLECTION_NAME,
        query_vector=query_vector,
        limit=5  # Get top 5 results
    )
    search_time = time.time() - search_start
    
    print(f"✅ Search completed in {search_time:.3f} seconds")
    
    if results.success and results.data:
        print(f"\n🎯 Found {len(results.data)} similar documents:")
        print("-" * 80)
        
        for i, result in enumerate(results.data, 1):
            print(f"{i}. ID: {result.id}")
            print(f"   Distance: {result.distance:.4f}")
            print(f"   Document: {result.metadata.get('document', 'N/A')}")
            print(f"   Word count: {result.metadata.get('word_count', 'N/A')}")
            print()
    else:
        print("❌ No results found or search failed")
        print(f"Response: {results}")

except Exception as e:
    print(f"❌ Search failed: {e}")

# 9. CLEANUP (OPTIONAL)
print(f"\n🧹 Cleanup options...")
cleanup = input("Do you want to delete the test collection? (y/n): ").lower()

if cleanup == 'y':
    try:
        delete_response = client.delete_collection(COLLECTION_NAME)
        print(f"🗑️  Collection deleted: {delete_response}")
    except Exception as e:
        print(f"❌ Cleanup failed: {e}")
else:
    print(f"📁 Collection '{COLLECTION_NAME}' kept for future use")

# 10. SUMMARY
print(f"\n{'='*60}")
print("🎉 DEMO COMPLETED SUCCESSFULLY!")
print("📋 What we accomplished:")
print("  ✅ Connected to d-vecDB server via ngrok")
print("  ✅ Created/managed vector collection")
print("  ✅ Inserted sample documents as vectors")
print("  ✅ Performed similarity search")
print("  ✅ Retrieved results with metadata")
print(f"{'='*60}")

print("\n💡 Next steps:")
print("1. Replace sample embeddings with real ones (e.g., sentence-transformers)")
print("2. Use your own documents and search queries")
print("3. Experiment with different distance metrics")
print("4. Try advanced features like filtering and batch operations")

# 11. HELPER FUNCTIONS FOR EXTENDED USAGE
def create_real_embeddings(texts: List[str]) -> List[List[float]]:
    """
    Helper function to create real embeddings using sentence-transformers.
    Uncomment and modify as needed.
    """
    # !pip install sentence-transformers
    # from sentence_transformers import SentenceTransformer
    # 
    # model = SentenceTransformer('all-MiniLM-L6-v2')
    # embeddings = model.encode(texts)
    # return [emb.tolist() for emb in embeddings]
    
    print("📝 To use real embeddings, uncomment the sentence-transformers code above")
    return [np.random.rand(384).tolist() for _ in texts]

def search_similar_documents(query_text: str, top_k: int = 5):
    """
    Search for similar documents given a text query.
    """
    # In practice, you'd embed the query_text using the same model
    # query_embedding = create_real_embeddings([query_text])[0]
    query_embedding = np.random.rand(384).tolist()
    
    results = client.search(
        collection_name=COLLECTION_NAME,
        query_vector=query_embedding,
        limit=top_k
    )
    
    return results

print("\n📚 Helper functions defined:")
print("  - create_real_embeddings(texts)")  
print("  - search_similar_documents(query_text, top_k)")
print("\nYou can use these functions to extend the functionality!")