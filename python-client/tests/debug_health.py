#!/usr/bin/env python3

import sys
print("Python path:")
for path in sys.path:
    print(f"  {path}")
print()

try:
    from vectordb_client.types import HealthResponse
    print("✅ Imported HealthResponse from vectordb_client.types")
    
    # Check the model fields
    print(f"Model fields: {HealthResponse.model_fields}")
    print()
    
    # Test creating with new format
    print("Testing HealthResponse with new format...")
    server_data = {'success': True, 'data': 'OK', 'error': None}
    try:
        health = HealthResponse(**server_data)
        print("✅ HealthResponse created successfully with new format")
        print(f"  health.success = {health.success}")
        print(f"  health.data = {health.data}")
        print(f"  health.error = {health.error}")
        print(f"  health.healthy = {health.healthy}")
        print(f"  health.status = {health.status}")
    except Exception as e:
        print(f"❌ Failed to create HealthResponse with new format: {e}")
    
    print()
    
    # Test creating with old format
    print("Testing HealthResponse with old format...")
    old_data = {'healthy': True, 'status': 'OK'}
    try:
        health = HealthResponse(**old_data)
        print("✅ HealthResponse created successfully with old format")
    except Exception as e:
        print(f"❌ Failed to create HealthResponse with old format: {e}")

except ImportError as e:
    print(f"❌ Failed to import HealthResponse: {e}")

# Now test the actual client
print("\n" + "="*50)
print("Testing VectorDBClient...")

try:
    from vectordb_client import VectorDBClient
    print("✅ Imported VectorDBClient")
    
    client = VectorDBClient(host='localhost', port=8080)
    print(f"✅ Created client: {type(client.client)}")
    
    print("Attempting ping...")
    result = client.ping()
    print(f"✅ Ping result: {result}")
    
except Exception as e:
    print(f"❌ Client error: {e}")
    import traceback
    traceback.print_exc()