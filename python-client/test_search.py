import numpy as np
from vectordb_client import VectorDBClient
from vectordb_client.types import SearchRequest

# Configuration
SERVER_HOST = "07604dfeb2ee.ngrok-free.app"
SERVER_PORT = 443
collection_name = "demo_collection"

print("🔍 Testing vector search...")

try:
    # Initialize client
    client = VectorDBClient(
        host=SERVER_HOST, 
        port=SERVER_PORT,
        ssl=True,
        protocol="rest"
    )
    
    # Create a query vector (normally this would be an embedding of your query text)
    query_vector = np.random.rand(384).tolist()
    
    # Perform search with individual parameters
    results = client.search(
        collection_name=collection_name,
        query_vector=query_vector,
        limit=3  # Get top 3 results
    )
    
    print(f"✅ Search completed!")
    print(f"📊 Results: {results}")
    
    # Display results nicely
    if results.success and results.data:
        print(f"\n🎯 Found {len(results.data)} similar vectors:")
        for i, result in enumerate(results.data, 1):
            print(f"  {i}. ID: {result.id}")
            print(f"     Distance: {result.distance:.4f}")
            if result.metadata:
                print(f"     Document: {result.metadata.get('document', 'N/A')}")
                print(f"     Word count: {result.metadata.get('word_count', 'N/A')}")
            print()

except Exception as e:
    print(f"❌ Search failed: {e}")
    print(f"Error type: {type(e).__name__}")