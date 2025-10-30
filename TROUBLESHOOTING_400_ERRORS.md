# Troubleshooting 400 Errors on Insert

**Problem:** Insert operations returning 400 Bad Request errors with no server logs.

---

## 🔍 Quick Diagnosis

Run the diagnostic tests to identify the issue:

```bash
# On your VPS
cd /root/d-vecDB

# Option 1: Bash test script
chmod +x test_insert.sh
./test_insert.sh 24.199.64.163 8080

# Option 2: Python test script
chmod +x test_insert.py
python3 test_insert.py 24.199.64.163 8080
```

These scripts will:
1. Check if server is accessible
2. Test single vector insert (auto ID)
3. Test single vector insert (with ID)
4. Test batch insert
5. Show which test fails and why

---

## 🐛 Common Causes of 400 Errors

### 1. **Wrong Vector Dimension**

**Problem:** Vector dimension doesn't match collection dimension.

**Server expects:**
```json
{
  "data": [0.1, 0.2, ...],  // Must be exactly `dimension` floats
  "metadata": {...}
}
```

**Check your collection dimension:**
```bash
curl http://24.199.64.163:8080/collections/incidents
# Look for: "dimension": 1536
```

**Fix:** Ensure your embeddings have exactly this many dimensions.

---

### 2. **Invalid JSON Format**

**Problem:** Request body doesn't match expected structure.

**Server expects for single insert:**
```json
{
  "id": "optional-uuid-string",
  "data": [0.1, 0.2, ...],
  "metadata": {
    "key": "value"
  }
}
```

**Server expects for batch insert:**
```json
{
  "vectors": [
    {
      "id": "optional-uuid",
      "data": [0.1, 0.2, ...],
      "metadata": {}
    },
    {
      "id": "optional-uuid",
      "data": [0.1, 0.2, ...],
      "metadata": {}
    }
  ]
}
```

**Common mistakes:**
- ❌ Missing `"data"` field
- ❌ `"data"` is not an array of numbers
- ❌ For batch: not wrapping in `"vectors"` array
- ❌ Metadata values contain functions or circular references

---

### 3. **Missing Content-Type Header**

**Problem:** Request doesn't include `Content-Type: application/json`

**Fix:**
```bash
curl -X POST http://24.199.64.163:8080/collections/incidents/vectors \
  -H "Content-Type: application/json" \  # ← Must include this!
  -d '{"data": [0.1, 0.2, ...]}'
```

**In Python:**
```python
response = requests.post(
    url,
    json=payload,  # This automatically sets Content-Type
    headers={"Content-Type": "application/json"}  # Or explicit
)
```

**In TypeScript:**
```typescript
const response = await axios.post(url, payload, {
  headers: {
    'Content-Type': 'application/json'
  }
});
```

---

### 4. **Invalid UUID Format**

**Problem:** If you provide an `id`, it must be a valid UUID.

**Valid UUID formats:**
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",  // ✅ Lowercase with dashes
  "id": "550E8400-E29B-41D4-A716-446655440000",  // ✅ Uppercase with dashes
  "data": [...]
}
```

**Invalid:**
```json
{
  "id": "not-a-uuid",           // ❌
  "id": "550e8400e29b41d4a716",  // ❌ Missing dashes
  "id": 123,                     // ❌ Not a string
  "data": [...]
}
```

**Best practice:** Don't provide `id` - let the server auto-generate it:
```json
{
  "data": [...],
  "metadata": {}
}
```

---

### 5. **NaN or Infinity in Embeddings**

**Problem:** Vector data contains NaN or Infinity values that can't serialize.

**Check in Python:**
```python
import numpy as np

# Check for NaN
if np.isnan(embedding).any():
    print("❌ Embedding contains NaN!")

# Check for Infinity
if np.isinf(embedding).any():
    print("❌ Embedding contains Infinity!")

# Fix by replacing
embedding = np.nan_to_num(embedding, nan=0.0, posinf=1.0, neginf=-1.0)
```

**Check in TypeScript:**
```typescript
// Check for NaN or Infinity
const hasInvalid = embedding.some(x => !isFinite(x));
if (hasInvalid) {
  console.error("❌ Embedding contains NaN or Infinity!");
}

// Fix by replacing
const cleaned = embedding.map(x =>
  isFinite(x) ? x : (isNaN(x) ? 0 : (x > 0 ? 1 : -1))
);
```

---

### 6. **Request Timeout**

**Problem:** Insert takes longer than timeout (30s single, 60s batch).

**Symptoms:**
- Not a 400 error, but a timeout error
- Server logs show "Operation timed out"

**Fix:**
- Reduce batch size (try 100-500 vectors at a time)
- Check server resources (CPU, memory)
- Check for long-running operations blocking the server

---

### 7. **Collection Doesn't Exist**

**Problem:** Trying to insert into non-existent collection.

**Check:**
```bash
curl http://24.199.64.163:8080/collections

# Response should list your collection:
{
  "success": true,
  "data": ["incidents", "users", ...]
}
```

**If missing, create it:**
```bash
curl -X POST http://24.199.64.163:8080/collections \
  -H "Content-Type: application/json" \
  -d '{
    "name": "incidents",
    "dimension": 1536,
    "distance_metric": "Cosine",
    "vector_type": "DenseFloat"
  }'
```

---

## 🔧 Debugging Steps

### Step 1: Verify Server is Running

```bash
# Check service status
sudo systemctl status d-vecdb

# If not running, start it
sudo systemctl start d-vecdb

# Check logs
sudo journalctl -u d-vecdb -f
```

### Step 2: Test with curl

```bash
# Simple health check
curl http://24.199.64.163:8080/health

# List collections
curl http://24.199.64.163:8080/collections

# Test minimal insert (replace DIMENSION with your collection's dimension)
curl -X POST http://24.199.64.163:8080/collections/incidents/vectors \
  -H "Content-Type: application/json" \
  -d "{\"data\": $(python3 -c 'import json; print(json.dumps([0.1] * 1536))')}"
```

### Step 3: Check Server Logs for Errors

```bash
# View recent logs
sudo journalctl -u d-vecdb -n 100

# Follow logs in real-time
sudo journalctl -u d-vecdb -f

# Look for errors
sudo journalctl -u d-vecdb | grep -i error
```

### Step 4: Verify Server Version

```bash
cd /root/d-vecDB
git log -1 --oneline

# Should show: 71a99e1 or later (with all fixes)
```

### Step 5: Test with Diagnostic Scripts

```bash
# Run comprehensive tests
cd /root/d-vecDB
./test_insert.py 24.199.64.163 8080

# This will tell you exactly which test fails
```

---

## 🎯 Most Likely Causes

Based on "400 errors with no server logs":

### **Cause #1: JSON Parsing Failure (90% probability)**

The request body doesn't match the expected structure, so axum's JSON extractor fails **before** the handler is called. This explains:
- ✅ 400 error (axum returns 400 for invalid JSON)
- ✅ No server logs (handler never runs)
- ✅ No detailed error message (axum doesn't provide details)

**How to confirm:**
```bash
# Enable TRACE logging
export RUST_LOG=trace
sudo systemctl restart d-vecdb

# Watch logs while making request
sudo journalctl -u d-vecdb -f
```

**Fix:** Check your request format matches exactly:
```json
{
  "data": [array of numbers],
  "metadata": {optional object}
}
```

### **Cause #2: Wrong Dimension (5% probability)**

Your embeddings have different dimension than collection expects.

**How to confirm:**
```bash
# Check collection dimension
curl http://24.199.64.163:8080/collections/incidents | jq '.data[0].dimension'

# Compare with your embedding dimension
python3 -c "print(len(your_embedding))"
```

**Fix:** Regenerate embeddings with correct dimension.

### **Cause #3: Content-Type Header Missing (3% probability)**

Request doesn't include `Content-Type: application/json`.

**How to confirm:** Check your HTTP client code.

**Fix:** Add header to all requests.

### **Cause #4: Server Not Running/Accessible (2% probability)**

**How to confirm:**
```bash
curl http://24.199.64.163:8080/health
```

**Fix:**
```bash
sudo systemctl start d-vecdb
```

---

## 🆘 Still Stuck?

If you've tried everything above and still getting 400 errors:

1. **Capture the exact request:**
   ```bash
   # Use curl with verbose output
   curl -v -X POST http://24.199.64.163:8080/collections/incidents/vectors \
     -H "Content-Type: application/json" \
     -d '{"data": [0.1, 0.2, ...]}' 2>&1 | tee request.log
   ```

2. **Enable maximum logging:**
   ```bash
   export RUST_LOG=trace,tower_http=debug,axum=trace
   sudo systemctl restart d-vecdb
   sudo journalctl -u d-vecdb -f
   ```

3. **Test with minimal example:**
   ```bash
   # Create minimal test collection
   curl -X POST http://24.199.64.163:8080/collections \
     -H "Content-Type: application/json" \
     -d '{
       "name": "test_minimal",
       "dimension": 2,
       "distance_metric": "Cosine",
       "vector_type": "DenseFloat"
     }'

   # Insert minimal vector
   curl -X POST http://24.199.64.163:8080/collections/test_minimal/vectors \
     -H "Content-Type: application/json" \
     -d '{"data": [0.1, 0.2]}'

   # If this works, your issue is with your data, not the server
   ```

4. **Share debug info:**
   - Output of `./test_insert.py`
   - Server logs: `sudo journalctl -u d-vecdb -n 200`
   - Server version: `git log -1 --oneline`
   - Request example (with actual data)
   - Your insertion code

---

## 📋 Quick Reference

### Working Insert (Single)
```bash
curl -X POST http://HOST:PORT/collections/NAME/vectors \
  -H "Content-Type: application/json" \
  -d '{"data": [0.1, 0.2, ..., 1536 floats], "metadata": {}}'
```

### Working Insert (Batch)
```bash
curl -X POST http://HOST:PORT/collections/NAME/vectors/batch \
  -H "Content-Type: application/json" \
  -d '{
    "vectors": [
      {"data": [0.1, ...], "metadata": {}},
      {"data": [0.2, ...], "metadata": {}}
    ]
  }'
```

### Check Collection Info
```bash
curl http://HOST:PORT/collections/NAME
```

### Check Server Health
```bash
curl http://HOST:PORT/health
```

### View Logs
```bash
sudo journalctl -u d-vecdb -f
```

---

**Last updated:** October 30, 2025
**Version:** v0.2.4
