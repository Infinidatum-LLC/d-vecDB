import sys
import os

# Add local client to Python path to use updated version
sys.path.insert(0, '/Users/durai/Documents/GitHub/d-vecDB/python-client')

import vectordb_client
from vectordb_client import VectorDBClient
from vectordb_client.types import (
    CollectionConfig, Vector, DistanceMetric,
    IndexConfig, VectorType
)

# Configuration - Replace with your server details
SERVER_HOST = "localhost"  # Replace with your server host
SERVER_PORT = 8080  # Replace with your server port

# For local development with ngrok, it might look like:
# SERVER_HOST = "abc123.ngrok.io"
# SERVER_PORT = 80

print(f"🔌 Connecting to d-vecDB server at {SERVER_HOST}:{SERVER_PORT}...")

try:
    # Initialize the client
    client = VectorDBClient(host=SERVER_HOST, port=SERVER_PORT)

    # Test the connection
    if client.ping():
        print("✅ Successfully connected to d-vecDB!")

        # Test health check
        health = client.health_check()
        print(f"📊 Health check: success={health.success}, data={health.data}")
    else:
        print("❌ Could not connect to d-vecDB server")
        print("Please check your server configuration and try again.")

except Exception as e:
    print(f"❌ Connection failed: {e}")
    print("\n💡 To run this example, you need:")
    print("1. A running d-vecDB server")
    print("2. Update SERVER_HOST and SERVER_PORT above")
    print("3. Ensure the server is accessible from Colab")