# Quick Start: Recover Your Lost Data

**Time to Recovery**: ~5 minutes
**Your Lost Data**: 78,796 embeddings (517 MB)
**Location**: `/root/d-vecdb-backup-20251029_074542/incidents/`

---

## ⚡ Fast Track Recovery (5 Steps)

### 1. Build the Recovery Patch (2 minutes)

```bash
cd /Users/durai/Documents/GitHub/d-vecDB

# Build release binary
cargo build --release --bin vectordb-server

# Wait for compilation...
# Binary will be at: target/release/vectordb-server
```

### 2. Deploy to Server (1 minute)

```bash
# Copy binary to server
scp -i ~/.ssh/id_digital \
    target/release/vectordb-server \
    root@64.227.20.45:/tmp/

# Install and restart
ssh -i ~/.ssh/id_digital root@64.227.20.45 << 'ENDSSH'
sudo systemctl stop d-vecdb
sudo mv /tmp/vectordb-server /usr/local/bin/
sudo chmod +x /usr/local/bin/vectordb-server
sudo systemctl start d-vecdb
sleep 5
sudo systemctl status d-vecdb
ENDSSH
```

### 3. Verify Recovery API Works (10 seconds)

```bash
# Test the new recovery API
curl http://64.227.20.45:3030/collections/deleted

# Expected: {"success": true, "data": []}
```

### 4. Delete Empty Collection (5 seconds)

```bash
# Remove the empty collection that's blocking import
curl -X DELETE http://64.227.20.45:3030/collections/incidents/hard-delete

# Expected: {"success": true, "data": {...}}
```

### 5. Import Your 78,796 Embeddings (2 minutes)

```bash
# Import the backup data
curl -X POST http://64.227.20.45:3030/collections/import \
  -H "Content-Type: application/json" \
  -d '{
    "orphaned_path": "/root/d-vecdb-backup-20251029_074542/incidents",
    "collection_name": "incidents",
    "dimension": 1536,
    "distance_metric": "Cosine",
    "vector_type": "Float32"
  }'

# Wait for import to complete (~2 minutes)

# Verify restoration
curl http://64.227.20.45:3030/collections/incidents

# Expected: {"success": true, "data": {"name": "incidents", "vector_count": 78796, ...}}
```

---

## ✅ Success! Your Data is Recovered

You just recovered 8 hours of embedding generation work in 5 minutes! 🎉

---

## 🛡️ Protect Future Data

### Enable Resilient Embedding Generation

```bash
cd /Users/durai/Documents/book-usecase/published/aiinflectionpoint-website

# Use new resilient script (already created for you)
python3 generate_embeddings_resilient.py

# Features:
# ✅ Automatic retry on connection failure
# ✅ Checkpoint every 10 records
# ✅ Resume from last position after crashes
# ✅ No more losing hours of work!
```

### Test Soft-Delete Protection

```bash
# Create a test collection
curl -X POST http://64.227.20.45:3030/collections \
  -H "Content-Type: application/json" \
  -d '{
    "name": "test",
    "dimension": 128,
    "distance_metric": "Cosine",
    "vector_type": "Float32"
  }'

# Delete it (soft-delete)
curl -X DELETE http://64.227.20.45:3030/collections/test

# Check it's in .deleted/
curl http://64.227.20.45:3030/collections/deleted

# Restore it
curl -X POST http://64.227.20.45:3030/collections/restore \
  -H "Content-Type: application/json" \
  -d '{
    "backup_path": "/data/.deleted/test_20251029_XXXXXX",
    "new_name": "test_restored"
  }'

# Verify restoration
curl http://64.227.20.45:3030/collections
```

---

## 📚 Full Documentation

- **Complete Guide**: `RECOVERY_PATCH_v0.2.0.md`
- **Implementation Details**: `RECOVERY_PATCH_SUMMARY.md`
- **Resilient Script**: `generate_embeddings_resilient.py`

---

## 🆘 Troubleshooting

### Build Fails?
```bash
# Check Rust version
rustc --version  # Should be 1.70+

# Update if needed
rustup update
```

### Can't Copy Binary to Server?
```bash
# Check SSH key
ls -la ~/.ssh/id_digital

# Test connection
ssh -i ~/.ssh/id_digital root@64.227.20.45 "echo OK"
```

### Import Fails?
```bash
# Verify backup exists
ssh -i ~/.ssh/id_digital root@64.227.20.45 \
  "ls -lh /root/d-vecdb-backup-20251029_074542/incidents/"

# Check server logs
ssh -i ~/.ssh/id_digital root@64.227.20.45 \
  "journalctl -u d-vecdb -n 50"
```

### API Returns Error?
```bash
# Check server is running
ssh -i ~/.ssh/id_digital root@64.227.20.45 \
  "sudo systemctl status d-vecdb"

# Check port is open
curl -v http://64.227.20.45:3030/health
```

---

## 🎯 What You Get

1. ✅ **78,796 Embeddings Recovered** - From backup in 2 minutes
2. ✅ **Soft-Delete Protection** - 24-hour recovery window for accidents
3. ✅ **Connection Resilience** - Never lose hours of work again
4. ✅ **Auto-Backup** - Before any destructive operation
5. ✅ **Peace of Mind** - Complete recovery system in place

---

**Ready? Start with Step 1 above!** 🚀
