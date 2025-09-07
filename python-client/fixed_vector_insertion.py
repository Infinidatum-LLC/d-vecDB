import time
import numpy as np
from vectordb_client import VectorDBClient
from vectordb_client.types import Vector

# Configuration
SERVER_HOST = "07604dfeb2ee.ngrok-free.app"
SERVER_PORT = 443
collection_name = "demo_collection"  # Use existing collection

# Sample data
sample_documents = [
    "The quick brown fox jumps over the lazy dog",
    "Machine learning is revolutionizing technology",
    "Vector databases enable semantic search capabilities",
    "Python is a versatile programming language",
    "Artificial intelligence transforms industries"
]

# Create sample embeddings (normally you'd use a real embedding model)
embedding_vectors = [
    np.random.rand(384).tolist() for _ in range(len(sample_documents))
]

print("🔌 Connecting to d-vecDB server...")

try:
    # Initialize client
    client = VectorDBClient(
        host=SERVER_HOST, 
        port=SERVER_PORT,
        ssl=True,
        protocol="rest"
    )
    
    print("✅ Connected successfully!")

    print("⬆️  Inserting vectors into collection...")

    try:
        # Prepare vectors with metadata - FIXED: Use 'data' instead of 'values'
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

        # Insert vectors in batch - FIXED: Use correct method name
        start_time = time.time()
        response = client.insert_vectors(collection_name, vectors_to_insert)  # FIXED: Changed from upsert_vectors
        insert_time = time.time() - start_time

        print(f"✅ Inserted {len(vectors_to_insert)} vectors in {insert_time:.2f} seconds")
        print(f"📊 Insert response: {response}")

        # Get collection statistics
        try:
            stats = client.get_collection_stats(collection_name)
            print(f"📈 Collection stats: {stats}")
        except Exception as stats_error:
            print(f"ℹ️  Collection stats not available: {stats_error}")

    except Exception as e:
        print(f"❌ Failed to insert vectors: {e}")
        print(f"Error type: {type(e).__name__}")
        
        # Try alternative method - insert_simple
        print("\n🔄 Trying alternative insert method...")
        try:
            for i, (doc, embedding) in enumerate(zip(sample_documents, embedding_vectors)):
                response = client.insert_simple(
                    collection_name=collection_name,
                    vector_id=str(i + 1),
                    vector_data=embedding,
                    metadata={
                        "document": doc,
                        "length": len(doc),
                        "index": i + 1,
                        "word_count": len(doc.split())
                    }
                )
                print(f"✅ Inserted vector {i+1}: {response}")
        except Exception as alt_error:
            print(f"❌ Alternative method also failed: {alt_error}")

except Exception as e:
    print(f"❌ Connection failed: {e}")

print("\n" + "="*60)
print("KEY FIXES MADE:")
print("1. Vector field: 'values' → 'data'")
print("2. Method name: 'upsert_vectors' → 'insert_vectors'")
print("3. Added alternative 'insert_simple' method")
print("="*60)