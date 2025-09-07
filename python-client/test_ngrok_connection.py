#!/usr/bin/env python3
"""
Test connection to ngrok d-vecDB server
"""

import vectordb_client
import asyncio
import json

# Server configuration
NGROK_URL = "https://07604dfeb2ee.ngrok-free.app"
HOST = "07604dfeb2ee.ngrok-free.app"
PORT = 443  # HTTPS port for ngrok

async def test_connection():
    """Test connection to the ngrok d-vecDB server"""
    
    print(f"Testing connection to {NGROK_URL}")
    print("-" * 50)
    
    try:
        # Create async client
        client = await vectordb_client.aconnect(
            host=HOST,
            port=PORT,
            protocol="rest",
            ssl=True
        )
        
        print("✅ Client created successfully")
        
        # Test server health
        print("\n1. Testing server health...")
        try:
            health = await client.health_check()
            print(f"✅ Server health: {health}")
        except Exception as e:
            print(f"❌ Health check failed: {e}")
        
        # Test listing collections
        print("\n2. Testing list collections...")
        try:
            collections = await client.list_collections()
            print(f"✅ Collections response: {collections}")
            if hasattr(collections, 'collections') and collections.collections:
                print(f"   Found {len(collections.collections)} collections:")
                for collection in collections.collections[:3]:  # Show first 3
                    print(f"   - {collection.name if hasattr(collection, 'name') else collection}")
        except Exception as e:
            print(f"❌ List collections failed: {e}")
        
        # Test server stats
        print("\n3. Testing server stats...")
        try:
            stats = await client.get_server_stats()
            print(f"✅ Server stats retrieved: {stats}")
        except Exception as e:
            print(f"❌ Server stats failed: {e}")
            
        # Test creating a simple collection
        print("\n4. Testing collection creation...")
        try:
            from vectordb_client.types import CollectionConfig, DistanceMetric
            config = CollectionConfig(
                name="test_collection",
                dimension=128,
                distance_metric=DistanceMetric.COSINE
            )
            result = await client.create_collection(config)
            print(f"✅ Collection created: {result}")
            
            # List collections again to verify
            collections = await client.list_collections()
            print(f"✅ Updated collections: {collections.data}")
            
        except Exception as e:
            print(f"❌ Collection creation failed: {e}")
            print(f"   This might be expected if collection already exists")
        
        print("\n✅ Connection test completed successfully!")
        
    except Exception as e:
        print(f"❌ Connection failed: {e}")
        print(f"   Error type: {type(e).__name__}")
        
        # Try with different configurations
        print("\n🔄 Trying alternative connection methods...")
        
        # Try with direct REST client
        try:
            from vectordb_client.rest.client import RestClient
            rest_client = RestClient(base_url=NGROK_URL)
            health = await rest_client.health()
            print(f"✅ Direct REST client health: {health}")
        except Exception as rest_e:
            print(f"❌ Direct REST client failed: {rest_e}")

if __name__ == "__main__":
    asyncio.run(test_connection())