#!/usr/bin/env python3
"""
Fixed connection example for d-vecDB with ngrok
"""

from vectordb_client import VectorDBClient
from vectordb_client.types import (
    CollectionConfig, Vector, DistanceMetric,
    IndexConfig, VectorType
)

# Configuration - Fixed for ngrok HTTPS
SERVER_HOST = "07604dfeb2ee.ngrok-free.app"
SERVER_PORT = 443  # HTTPS port for ngrok
USE_SSL = True     # Required for ngrok HTTPS URLs

print(f"🔌 Connecting to d-vecDB server at {SERVER_HOST}:{SERVER_PORT} (SSL: {USE_SSL})...")

try:
    # Initialize the client with correct configuration
    client = VectorDBClient(
        host=SERVER_HOST, 
        port=SERVER_PORT,
        protocol="rest",
        ssl=USE_SSL
    )

    print("✅ Client initialized successfully!")

    # Test the connection with correct method names
    print("\n🏥 Testing server health...")
    health = client.health_check()
    print(f"✅ Server health: {health}")

    # Get server stats (not server info)
    print("\n📊 Getting server statistics...")
    server_stats = client.get_server_stats()
    print(f"📈 Server Stats: {server_stats}")

    # List collections
    print("\n📂 Listing collections...")
    collections = client.list_collections()
    print(f"📋 Collections: {collections.data}")

    print("\n🎉 All tests passed! Connection is working perfectly.")

except AttributeError as ae:
    print(f"❌ Method not found: {ae}")
    print("💡 This might be because the method name is incorrect.")
    print("   Try using: health_check(), get_server_stats(), list_collections()")

except ConnectionError as ce:
    print(f"❌ Connection failed: {ce}")
    print("💡 Check if:")
    print("  1. The ngrok tunnel is still active")
    print("  2. The d-vecDB server is running")
    print("  3. The host URL is correct")

except Exception as e:
    print(f"❌ Unexpected error: {e}")
    print(f"   Error type: {type(e).__name__}")
    print("\n💡 Troubleshooting tips:")
    print("1. Verify your ngrok URL is correct and active")
    print("2. Make sure you're using port 443 for HTTPS ngrok URLs")
    print("3. Ensure SSL=True for HTTPS connections")
    print("4. Check that the d-vecDB server is running and accessible")

print("\n" + "="*60)
print("CORRECT CONFIGURATION FOR NGROK:")
print("- Host: your-ngrok-url.ngrok-free.app (without https://)")
print("- Port: 443 (for HTTPS)")
print("- SSL: True")
print("- Protocol: 'rest'")
print("="*60)