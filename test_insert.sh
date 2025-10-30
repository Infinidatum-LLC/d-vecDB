#!/bin/bash

# d-vecDB Insert Diagnostic Test
# Run this on your VPS to diagnose insert issues

set -e

HOST="${1:-24.199.64.163}"
PORT="${2:-8080}"
BASE_URL="http://$HOST:$PORT"

echo "================================================"
echo "d-vecDB Insert Diagnostic Test"
echo "================================================"
echo "Server: $BASE_URL"
echo ""

# Test 1: Check if server is accessible
echo "Test 1: Checking server accessibility..."
if curl -s -f "$BASE_URL/health" > /dev/null 2>&1; then
    echo "✅ Server is accessible"
else
    echo "❌ Server is NOT accessible"
    echo "   Check if server is running: sudo systemctl status d-vecdb"
    exit 1
fi
echo ""

# Test 2: List collections
echo "Test 2: Listing collections..."
COLLECTIONS=$(curl -s "$BASE_URL/collections")
echo "Response: $COLLECTIONS"

# Extract first collection name
COLLECTION_NAME=$(echo "$COLLECTIONS" | jq -r '.data[0]' 2>/dev/null || echo "incidents")
echo "Using collection: $COLLECTION_NAME"
echo ""

# Test 3: Get collection info
echo "Test 3: Getting collection info..."
COLLECTION_INFO=$(curl -s "$BASE_URL/collections/$COLLECTION_NAME")
echo "Response: $COLLECTION_INFO"

# Extract dimension
DIMENSION=$(echo "$COLLECTION_INFO" | jq -r '.data[0].dimension' 2>/dev/null || echo "1536")
echo "Dimension: $DIMENSION"
echo ""

# Test 4: Insert a single vector (simple test)
echo "Test 4: Testing single vector insert..."
echo "Creating test vector with dimension $DIMENSION..."

# Generate a test vector with the correct dimension
TEST_VECTOR=$(python3 -c "import json; print(json.dumps([0.1] * $DIMENSION))")

# Test insert WITHOUT id (auto-generate)
echo "Attempting insert WITHOUT id (auto-generate)..."
INSERT_RESPONSE=$(curl -s -w "\nHTTP_STATUS:%{http_code}" \
  -X POST "$BASE_URL/collections/$COLLECTION_NAME/vectors" \
  -H "Content-Type: application/json" \
  -d "{
    \"data\": $TEST_VECTOR,
    \"metadata\": {\"test\": true, \"timestamp\": \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\"}
  }")

HTTP_STATUS=$(echo "$INSERT_RESPONSE" | grep "HTTP_STATUS:" | cut -d: -f2)
RESPONSE_BODY=$(echo "$INSERT_RESPONSE" | grep -v "HTTP_STATUS:")

echo "HTTP Status: $HTTP_STATUS"
echo "Response Body: $RESPONSE_BODY"

if [ "$HTTP_STATUS" = "200" ]; then
    echo "✅ Insert succeeded!"
else
    echo "❌ Insert failed with status $HTTP_STATUS"

    # Try to parse error message
    ERROR_MSG=$(echo "$RESPONSE_BODY" | jq -r '.error' 2>/dev/null || echo "Unknown error")
    echo "Error: $ERROR_MSG"
fi
echo ""

# Test 5: Insert with explicit UUID
echo "Test 5: Testing insert WITH explicit UUID..."
TEST_UUID="$(uuidgen | tr '[:upper:]' '[:lower:]')"
echo "Using UUID: $TEST_UUID"

INSERT_RESPONSE2=$(curl -s -w "\nHTTP_STATUS:%{http_code}" \
  -X POST "$BASE_URL/collections/$COLLECTION_NAME/vectors" \
  -H "Content-Type: application/json" \
  -d "{
    \"id\": \"$TEST_UUID\",
    \"data\": $TEST_VECTOR,
    \"metadata\": {\"test\": true, \"has_id\": true}
  }")

HTTP_STATUS2=$(echo "$INSERT_RESPONSE2" | grep "HTTP_STATUS:" | cut -d: -f2)
RESPONSE_BODY2=$(echo "$INSERT_RESPONSE2" | grep -v "HTTP_STATUS:")

echo "HTTP Status: $HTTP_STATUS2"
echo "Response Body: $RESPONSE_BODY2"

if [ "$HTTP_STATUS2" = "200" ]; then
    echo "✅ Insert with UUID succeeded!"
else
    echo "❌ Insert with UUID failed with status $HTTP_STATUS2"
fi
echo ""

# Test 6: Batch insert (small batch)
echo "Test 6: Testing batch insert (2 vectors)..."
UUID1="$(uuidgen | tr '[:upper:]' '[:lower:]')"
UUID2="$(uuidgen | tr '[:upper:]' '[:lower:]')"

BATCH_RESPONSE=$(curl -s -w "\nHTTP_STATUS:%{http_code}" \
  -X POST "$BASE_URL/collections/$COLLECTION_NAME/vectors/batch" \
  -H "Content-Type: application/json" \
  -d "{
    \"vectors\": [
      {
        \"id\": \"$UUID1\",
        \"data\": $TEST_VECTOR,
        \"metadata\": {\"batch\": true, \"index\": 0}
      },
      {
        \"id\": \"$UUID2\",
        \"data\": $TEST_VECTOR,
        \"metadata\": {\"batch\": true, \"index\": 1}
      }
    ]
  }")

HTTP_STATUS3=$(echo "$BATCH_RESPONSE" | grep "HTTP_STATUS:" | cut -d: -f2)
RESPONSE_BODY3=$(echo "$BATCH_RESPONSE" | grep -v "HTTP_STATUS:")

echo "HTTP Status: $HTTP_STATUS3"
echo "Response Body: $RESPONSE_BODY3"

if [ "$HTTP_STATUS3" = "200" ]; then
    echo "✅ Batch insert succeeded!"
else
    echo "❌ Batch insert failed with status $HTTP_STATUS3"
fi
echo ""

# Test 7: Check collection stats
echo "Test 7: Checking collection stats after inserts..."
STATS=$(curl -s "$BASE_URL/collections/$COLLECTION_NAME")
VECTOR_COUNT=$(echo "$STATS" | jq -r '.data[1].vector_count' 2>/dev/null || echo "unknown")
echo "Vector count: $VECTOR_COUNT"

if [ "$VECTOR_COUNT" != "unknown" ] && [ "$VECTOR_COUNT" != "0" ]; then
    echo "✅ Vectors are being indexed!"
else
    echo "⚠️  Vector count is 0 or unknown"
fi
echo ""

# Summary
echo "================================================"
echo "Test Summary"
echo "================================================"

if [ "$HTTP_STATUS" = "200" ] && [ "$HTTP_STATUS2" = "200" ] && [ "$HTTP_STATUS3" = "200" ]; then
    echo "✅ ALL TESTS PASSED"
    echo ""
    echo "The d-vecDB server is working correctly!"
    echo "If your application is getting 400 errors, check:"
    echo "  1. Request format matches server expectations"
    echo "  2. Vector dimension matches collection dimension ($DIMENSION)"
    echo "  3. Content-Type header is 'application/json'"
    echo "  4. JSON is valid and properly formatted"
else
    echo "❌ SOME TESTS FAILED"
    echo ""
    echo "Next steps:"
    echo "  1. Check server logs: sudo journalctl -u d-vecdb -n 100"
    echo "  2. Verify server is running: sudo systemctl status d-vecdb"
    echo "  3. Check server version: cd /root/d-vecDB && git log -1 --oneline"
    echo "  4. Try restarting: sudo systemctl restart d-vecdb"
fi
echo ""

# Check server logs for errors
echo "Recent server logs (last 20 lines):"
echo "================================================"
if command -v journalctl &> /dev/null; then
    sudo journalctl -u d-vecdb -n 20 --no-pager 2>/dev/null || echo "Cannot access journalctl"
else
    echo "journalctl not available"
fi
