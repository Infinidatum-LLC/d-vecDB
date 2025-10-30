# Quick Fix Reference - Embeddings Not Recognized After Restart

**TL;DR:** Your vectors are stored but not being loaded into the search index due to a storage format mismatch. Here's the fastest way to fix it.

---

## 🚨 The Problem

```bash
# Symptoms:
✅ vectors.bin exists (515 MB)
✅ Collection exists
❌ Search returns []
❌ Stats show vector_count: 0
❌ Logs show "Loaded 0 vectors from storage"
```

**Root Cause:** Your old vectors.bin uses format A, but new code expects format B.

---

## ⚡ Fastest Fix (5 Minutes)

### If You Have Original Embeddings

```bash
# On your VPS
cd /root/d-vecDB
git pull origin master
cargo build --release --bin vectordb-server

# Delete old collection
sudo systemctl stop d-vecdb
sudo rm -rf /root/embedding-project/dvecdb-data/incidents

# Start server
sudo systemctl start d-vecdb

# Re-insert embeddings (using your original code)
python3 your_embedding_script.py
```

Done! ✅

---

## 🔄 Alternative: Try Migration (10 Minutes)

### If You DON'T Have Original Embeddings

```bash
# On your VPS
cd /root/d-vecDB
git pull origin master
cargo build --release --bin vectordb-server

# Install rust-script
cargo install rust-script

# Backup first!
sudo cp -r /root/embedding-project/dvecdb-data \
           /root/embedding-project/dvecdb-data.backup

# Try migration
./migrate_vectors.rs /root/embedding-project/dvecdb-data/incidents

# Start server
sudo systemctl start d-vecdb

# Check logs
sudo journalctl -u d-vecdb -f | grep "Loaded"
```

**Expected output:**
```
[INFO] Loaded 78796 vectors from storage for collection 'incidents'
[INFO] Successfully rebuilt index with 78796 vectors
```

If you see "Loaded 0 vectors", the migration failed → Use the first option (re-insert).

---

## ✅ Verify It Works

```bash
# Test 1: Check stats
curl http://24.199.64.163:8080/collections/incidents
# Should show: "vector_count": 78796

# Test 2: Search
curl -X POST http://24.199.64.163:8080/collections/incidents/search \
  -H "Content-Type: application/json" \
  -d '{"query_vector": [0.1, 0.2, ...], "limit": 5}'
# Should return results (not empty array)

# Test 3: Restart
sudo systemctl restart d-vecdb
# Check logs - should still show "Loaded 78796 vectors"
```

---

## 🔍 Debug Command

If something goes wrong:

```bash
cd /root/d-vecDB
./debug_vectors.sh /root/embedding-project/dvecdb-data incidents
```

This will tell you:
- If files exist
- File format (old vs new)
- What the logs say

---

## 📚 Full Documentation

For detailed explanation and troubleshooting:

1. **Storage format issue:** Read `STORAGE_FORMAT_FIX.md`
2. **Index rebuild issue:** Read `INDEX_REBUILD_FIX_v0.2.3.md`
3. **Deployment guide:** Read `DEPLOYMENT_GUIDE_v0.2.4.md`

---

## 🆘 Still Broken?

1. **Check you have latest code:**
   ```bash
   cd /root/d-vecDB
   git log -1 --oneline
   # Should show: cbe6c3c or later
   ```

2. **Check you rebuilt:**
   ```bash
   ls -lh /root/d-vecDB/target/release/vectordb-server
   # Should show recent timestamp (today)
   ```

3. **Check logs for errors:**
   ```bash
   sudo journalctl -u d-vecdb -n 100
   ```

4. **Create GitHub issue** with:
   - Output of `./debug_vectors.sh`
   - Last 100 lines of logs
   - What you tried

---

## 🎯 What Changed?

| Before | After |
|--------|-------|
| vectors.bin: `[data][data][data]` | vectors.bin: `[len][data][len][data]` |
| Reader expects length prefixes | Writer now includes length prefixes ✅ |
| Reader finds garbage → 0 vectors | Reader finds real data → N vectors ✅ |

Your old file is missing the length prefixes, so the new code can't read it.

**Solution:** Either re-insert or migrate.

---

**Last updated:** October 30, 2025
**Version:** v0.2.4
**Commits:** 38ed709, c2dd7c2, c6a424f, cbe6c3c
