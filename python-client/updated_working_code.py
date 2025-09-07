from vectordb_client import VectorDBClient
from vectordb_client.types import (
    CollectionConfig, Vector, DistanceMetric,
    IndexConfig, VectorType
)

# Configuration - Updated for ngrok HTTPS
SERVER_HOST = "07604dfeb2ee.ngrok-free.app"  # Your ngrok host
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