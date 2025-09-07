from vectordb_client import VectorDBClient
from vectordb_client.types import (
    CollectionConfig, Vector, DistanceMetric,
    IndexConfig, VectorType
)

# Configuration - FIXED for ngrok
SERVER_HOST = "07604dfeb2ee.ngrok-free.app"  # Your ngrok host (correct)
SERVER_PORT = 443  # FIXED: Use 443 for HTTPS ngrok URLs, not 8080

# For local development with ngrok HTTPS, it should be:
# SERVER_HOST = "abc123.ngrok-free.app"  # No https:// prefix
# SERVER_PORT = 443  # HTTPS port

print(f"🔌 Connecting to d-vecDB server at {SERVER_HOST}:{SERVER_PORT}...")

try:
    # Initialize the client - FIXED: Added SSL support for ngrok
    client = VectorDBClient(
        host=SERVER_HOST, 
        port=SERVER_PORT,
        ssl=True,  # REQUIRED for ngrok HTTPS URLs
        protocol="rest"
    )

    # Test the connection - FIXED: Use correct method name
    print("🏥 Testing connection...")
    health_response = client.health_check()  # FIXED: ping() doesn't exist, use health_check()
    
    if health_response and health_response.success:
        print("✅ Successfully connected to d-vecDB!")

        # Get server info - FIXED: Use correct method name
        server_stats = client.get_server_stats()  # FIXED: get_server_info() doesn't exist
        print(f"📊 Server Stats: {server_stats}")
        
        # List existing collections
        collections = client.list_collections()
        print(f"📂 Collections: {collections.data}")
        
    else:
        print("❌ Could not connect to d-vecDB server")
        print("Please check your server configuration and try again.")

except Exception as e:
    print(f"❌ Connection failed: {e}")
    print(f"Error type: {type(e).__name__}")
    print("\n💡 To run this example, you need:")
    print("1. A running d-vecDB server")
    print("2. Correct ngrok configuration:")
    print(f"   - Host: {SERVER_HOST}")
    print(f"   - Port: {SERVER_PORT} (443 for HTTPS)")
    print("   - SSL: True (required for ngrok HTTPS)")
    print("3. Ensure the server is accessible from your environment")