#!/usr/bin/env python3
"""
d-vecDB Insert Diagnostic Test (Python)
Run this on your VPS to diagnose insert issues
"""

import requests
import json
import sys
import uuid
from datetime import datetime

# Configuration
HOST = sys.argv[1] if len(sys.argv) > 1 else "24.199.64.163"
PORT = sys.argv[2] if len(sys.argv) > 2 else "8080"
BASE_URL = f"http://{HOST}:{PORT}"

print("=" * 60)
print("d-vecDB Insert Diagnostic Test (Python)")
print("=" * 60)
print(f"Server: {BASE_URL}")
print()

def test_health():
    """Test 1: Check if server is accessible"""
    print("Test 1: Checking server accessibility...")
    try:
        response = requests.get(f"{BASE_URL}/health", timeout=5)
        if response.status_code == 200:
            print("✅ Server is accessible")
            return True
        else:
            print(f"❌ Server returned status {response.status_code}")
            return False
    except Exception as e:
        print(f"❌ Server is NOT accessible: {e}")
        print("   Check if server is running: sudo systemctl status d-vecdb")
        return False

def test_list_collections():
    """Test 2: List collections"""
    print("\nTest 2: Listing collections...")
    try:
        response = requests.get(f"{BASE_URL}/collections")
        print(f"Response: {response.json()}")

        data = response.json()
        if data.get("success") and data.get("data"):
            collection_name = data["data"][0]
            print(f"Using collection: {collection_name}")
            return collection_name
        else:
            print("⚠️  No collections found, using 'incidents'")
            return "incidents"
    except Exception as e:
        print(f"❌ Error: {e}")
        return "incidents"

def test_get_collection_info(collection_name):
    """Test 3: Get collection info"""
    print(f"\nTest 3: Getting collection info for '{collection_name}'...")
    try:
        response = requests.get(f"{BASE_URL}/collections/{collection_name}")
        print(f"Response: {response.json()}")

        data = response.json()
        if data.get("success") and data.get("data"):
            dimension = data["data"][0]["dimension"]
            print(f"Dimension: {dimension}")
            return dimension
        else:
            print("⚠️  Could not get dimension, using 1536")
            return 1536
    except Exception as e:
        print(f"❌ Error: {e}")
        return 1536

def test_single_insert(collection_name, dimension):
    """Test 4: Insert a single vector (auto-generate ID)"""
    print(f"\nTest 4: Testing single vector insert (auto-generate ID)...")
    print(f"Creating test vector with dimension {dimension}...")

    test_vector = [0.1] * dimension
    payload = {
        "data": test_vector,
        "metadata": {
            "test": True,
            "timestamp": datetime.utcnow().isoformat()
        }
    }

    try:
        response = requests.post(
            f"{BASE_URL}/collections/{collection_name}/vectors",
            json=payload,
            headers={"Content-Type": "application/json"},
            timeout=30
        )

        print(f"HTTP Status: {response.status_code}")
        print(f"Response Body: {response.json()}")

        if response.status_code == 200:
            print("✅ Insert succeeded!")
            return True
        else:
            print(f"❌ Insert failed with status {response.status_code}")
            if response.json().get("error"):
                print(f"Error: {response.json()['error']}")
            return False
    except Exception as e:
        print(f"❌ Exception: {e}")
        return False

def test_insert_with_id(collection_name, dimension):
    """Test 5: Insert with explicit UUID"""
    print(f"\nTest 5: Testing insert WITH explicit UUID...")

    test_uuid = str(uuid.uuid4())
    print(f"Using UUID: {test_uuid}")

    test_vector = [0.2] * dimension
    payload = {
        "id": test_uuid,
        "data": test_vector,
        "metadata": {
            "test": True,
            "has_id": True
        }
    }

    try:
        response = requests.post(
            f"{BASE_URL}/collections/{collection_name}/vectors",
            json=payload,
            headers={"Content-Type": "application/json"},
            timeout=30
        )

        print(f"HTTP Status: {response.status_code}")
        print(f"Response Body: {response.json()}")

        if response.status_code == 200:
            print("✅ Insert with UUID succeeded!")
            return True
        else:
            print(f"❌ Insert with UUID failed with status {response.status_code}")
            return False
    except Exception as e:
        print(f"❌ Exception: {e}")
        return False

def test_batch_insert(collection_name, dimension):
    """Test 6: Batch insert (2 vectors)"""
    print(f"\nTest 6: Testing batch insert (2 vectors)...")

    uuid1 = str(uuid.uuid4())
    uuid2 = str(uuid.uuid4())

    test_vector = [0.3] * dimension
    payload = {
        "vectors": [
            {
                "id": uuid1,
                "data": test_vector,
                "metadata": {"batch": True, "index": 0}
            },
            {
                "id": uuid2,
                "data": test_vector,
                "metadata": {"batch": True, "index": 1}
            }
        ]
    }

    try:
        response = requests.post(
            f"{BASE_URL}/collections/{collection_name}/vectors/batch",
            json=payload,
            headers={"Content-Type": "application/json"},
            timeout=60
        )

        print(f"HTTP Status: {response.status_code}")
        print(f"Response Body: {response.json()}")

        if response.status_code == 200:
            print("✅ Batch insert succeeded!")
            return True
        else:
            print(f"❌ Batch insert failed with status {response.status_code}")
            return False
    except Exception as e:
        print(f"❌ Exception: {e}")
        return False

def test_collection_stats(collection_name):
    """Test 7: Check collection stats"""
    print(f"\nTest 7: Checking collection stats after inserts...")
    try:
        response = requests.get(f"{BASE_URL}/collections/{collection_name}")
        data = response.json()

        if data.get("success") and data.get("data"):
            vector_count = data["data"][1].get("vector_count", "unknown")
            print(f"Vector count: {vector_count}")

            if vector_count != "unknown" and vector_count > 0:
                print("✅ Vectors are being indexed!")
                return True
            else:
                print("⚠️  Vector count is 0 or unknown")
                return False
        else:
            print("⚠️  Could not get stats")
            return False
    except Exception as e:
        print(f"❌ Error: {e}")
        return False

def main():
    """Run all tests"""
    results = []

    # Test 1: Health check
    if not test_health():
        print("\n❌ Server not accessible, stopping tests")
        return False

    # Test 2: List collections
    collection_name = test_list_collections()

    # Test 3: Get collection info
    dimension = test_get_collection_info(collection_name)

    # Test 4: Single insert (auto ID)
    results.append(("Single insert (auto ID)", test_single_insert(collection_name, dimension)))

    # Test 5: Single insert (with ID)
    results.append(("Single insert (with ID)", test_insert_with_id(collection_name, dimension)))

    # Test 6: Batch insert
    results.append(("Batch insert", test_batch_insert(collection_name, dimension)))

    # Test 7: Stats check
    test_collection_stats(collection_name)

    # Summary
    print("\n" + "=" * 60)
    print("Test Summary")
    print("=" * 60)

    all_passed = all(result for _, result in results)

    if all_passed:
        print("✅ ALL TESTS PASSED")
        print()
        print("The d-vecDB server is working correctly!")
        print("If your application is getting 400 errors, check:")
        print(f"  1. Request format matches server expectations")
        print(f"  2. Vector dimension matches collection dimension ({dimension})")
        print("  3. Content-Type header is 'application/json'")
        print("  4. JSON is valid and properly formatted")
        print("  5. Embeddings don't contain NaN or Infinity values")
    else:
        print("❌ SOME TESTS FAILED")
        print()
        print("Failed tests:")
        for name, result in results:
            if not result:
                print(f"  ❌ {name}")
        print()
        print("Next steps:")
        print("  1. Check server logs: sudo journalctl -u d-vecdb -n 100")
        print("  2. Verify server is running: sudo systemctl status d-vecdb")
        print("  3. Check server version: cd /root/d-vecDB && git log -1 --oneline")
        print("  4. Try rebuilding: cd /root/d-vecDB && cargo build --release")
        print("  5. Try restarting: sudo systemctl restart d-vecdb")

    print()
    return all_passed

if __name__ == "__main__":
    try:
        success = main()
        sys.exit(0 if success else 1)
    except KeyboardInterrupt:
        print("\n\n❌ Test interrupted by user")
        sys.exit(1)
    except Exception as e:
        print(f"\n\n❌ Unexpected error: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)
