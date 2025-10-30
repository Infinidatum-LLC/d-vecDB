# Deployment Guide - d-vecDB v0.2.4 Critical Fixes

**Date:** October 30, 2025
**Status:** 🚀 Ready to Deploy
**Severity:** 🔴 CRITICAL - Fixes embeddings not being recognized after restart

---

## 🎯 What's Fixed

This release includes **THREE CRITICAL FIXES**:

1. ✅ **Metadata Persistence** (v0.2.1, commit 38ed709)
   - Collections now survive server restarts via `metadata.json`
   - Automatic collection discovery on startup

2. ✅ **Index Rebuild** (v0.2.3, commit c2dd7c2)
   - Vectors now loaded from storage into HNSW index on restart
   - Fixed "empty index" bug that caused search to return `[]`

3. ✅ **Storage Format Fix** (v0.2.4, commit c6a424f)
   - Fixed format mismatch between write and read operations
   - Added length prefixes to vector storage format
   - **BREAKING CHANGE:** Old vectors.bin files need migration

---

## 🚀 Quick Deployment (VPS)

### Step 1: Backup Your Data

```bash
# Stop the server first
sudo systemctl stop d-vecdb

# Backup your data directory
sudo cp -r /root/embedding-project/dvecdb-data \
           /root/embedding-project/dvecdb-data.backup.$(date +%Y%m%d-%H%M%S)

# Verify backup
ls -lh /root/embedding-project/dvecdb-data.backup.*
```

### Step 2: Pull Latest Code

```bash
cd /root/d-vecDB
git pull origin master

# Verify you have all the fixes
git log --oneline -10
# Should show:
# cbe6c3c fix(migrate): Add cargo dependencies and make scripts executable
# 1af288f docs: Add comprehensive storage format fix documentation
# c6a424f fix(storage): Add length prefixes to vector storage format
# 2913655 debug: Add vector loading debug script
# 30f99f9 docs: Add comprehensive documentation...
# c2dd7c2 fix(critical): Index rebuild now loads vectors...
```

### Step 3: Rebuild Server

```bash
cd /root/d-vecDB
cargo build --release --bin vectordb-server

# This will take a few minutes...
# Should complete without errors
```

### Step 4: Choose Your Migration Path

**⚠️ IMPORTANT:** Your existing `vectors.bin` file (515MB) uses the OLD FORMAT and won't work with the new code. You must choose one of these options:

#### Option A: Re-Insert Embeddings (RECOMMENDED)

If you have the original embeddings/data:

```bash
# Start the new server
sudo systemctl start d-vecdb

# Re-insert your embeddings using your original insertion code
# Example (using Python client):
python3 your_insertion_script.py

# Or using TypeScript client:
node your_insertion_script.js
```

**Pros:**
- Clean, guaranteed to work
- Uses new format from the start
- No migration complexity

**Cons:**
- Requires original embeddings
- Takes time to re-insert 78K vectors (~5-10 minutes)

#### Option B: Try Migration Script (EXPERIMENTAL)

If you DON'T have original embeddings:

```bash
# Install rust-script (one-time setup)
cargo install rust-script

# Test migration on a copy first
cp -r /root/embedding-project/dvecdb-data/incidents \
      /root/embedding-project/dvecdb-data/incidents_test

# Run migration on test copy
cd /root/d-vecDB
./migrate_vectors.rs /root/embedding-project/dvecdb-data/incidents_test

# If successful, migrate the real data
./migrate_vectors.rs /root/embedding-project/dvecdb-data/incidents

# Start server
sudo systemctl start d-vecdb
```

**Pros:**
- Keeps original vectors
- Automated process

**Cons:**
- Experimental - may not work if old format is incompatible
- Risk of data corruption (backup first!)

#### Option C: Start Fresh

If you're okay starting over:

```bash
# Move old data out of the way
sudo mv /root/embedding-project/dvecdb-data \
        /root/embedding-project/dvecdb-data.old

# Create fresh directory
sudo mkdir -p /root/embedding-project/dvecdb-data

# Start server
sudo systemctl start d-vecdb

# Re-insert vectors via API
```

### Step 5: Verify the Fix

After starting the server with your chosen option:

```bash
# Check server logs
sudo journalctl -u d-vecdb -f | grep -E "(Loaded|Successfully|ERROR)"

# You should see:
# [INFO] Discovered collection: incidents
# [INFO] Loaded 78796 vectors from storage for collection 'incidents'
# [INFO] Successfully rebuilt index for collection 'incidents' with 78796 vectors
```

**Test search:**

```bash
curl -X POST http://24.199.64.163:8080/collections/incidents/search \
  -H "Content-Type: application/json" \
  -d '{
    "query_vector": [0.1, 0.2, ...],
    "limit": 5
  }'

# Should return results, NOT empty array!
```

**Check stats:**

```bash
curl http://24.199.64.163:8080/collections/incidents

# Should show:
# "vector_count": 78796  (not 0!)
```

---

## 🔍 Troubleshooting

### Problem: Still seeing "Loaded 0 vectors"

**Check 1:** Verify you have the latest code
```bash
cd /root/d-vecDB
git log -1 --oneline
# Should show: cbe6c3c or later
```

**Check 2:** Did you rebuild?
```bash
./target/release/vectordb-server --version
# Should be recent build date
```

**Check 3:** Check file format
```bash
cd /root/d-vecDB
./debug_vectors.sh /root/embedding-project/dvecdb-data incidents
```

This script will analyze your vectors.bin file and tell you if it's in the old or new format.

### Problem: Migration Script Fails

```bash
# Check error messages
./migrate_vectors.rs /path/to/collection 2>&1 | tee migration.log

# If it says "No vectors could be read":
# Your old file uses an incompatible format
# → Use Option A (re-insert) or Option C (start fresh)
```

### Problem: Server Crashes

```bash
# Check logs for errors
sudo journalctl -u d-vecdb -n 100

# Common issues:
# 1. Out of memory → Add more RAM or reduce batch size
# 2. Permission errors → Check file ownership
# 3. Disk full → Free up space
```

---

## 📊 What to Expect

### Startup Time

With 78,796 vectors, expect:
- **First startup (rebuilding index):** 5-10 seconds
- **Normal startup (after index built):** < 1 second

### Memory Usage

During startup:
- Loads all vectors into memory temporarily
- Inserts into HNSW index
- Peak memory: ~550 MB for 78K vectors (1536 dim)

### Search Performance

After restart:
- ✅ Search should work immediately
- ✅ Results should match pre-restart behavior
- ✅ No "warm-up" period needed

---

## 🎉 Success Criteria

You'll know it's working when:

1. ✅ Server starts without errors
2. ✅ Logs show "Loaded N vectors from storage"
3. ✅ Logs show "Successfully rebuilt index with N vectors"
4. ✅ `/collections/incidents` shows correct `vector_count`
5. ✅ Search returns results (not empty array)
6. ✅ After restart, everything still works

---

## 📝 Summary of Changes

| Commit | Description | Impact |
|--------|-------------|--------|
| 38ed709 | Metadata persistence | Collections survive restarts |
| c2dd7c2 | Index rebuild fix | Vectors loaded into index on startup |
| c6a424f | Storage format fix | Write/read format now consistent |
| cbe6c3c | Migration script fix | Script now includes dependencies |

---

## 🆘 Need Help?

If you encounter issues:

1. **Check logs:** `sudo journalctl -u d-vecdb -f`
2. **Run debug script:** `./debug_vectors.sh /path/to/data collection_name`
3. **Create GitHub issue:** Include:
   - Output of debug script
   - Server logs (last 100 lines)
   - Which migration option you tried
   - Error messages

---

## 🔄 Rollback Plan (If Needed)

If the new version causes issues:

```bash
# Stop new server
sudo systemctl stop d-vecdb

# Restore old data
sudo rm -rf /root/embedding-project/dvecdb-data
sudo mv /root/embedding-project/dvecdb-data.backup.YYYYMMDD-HHMMSS \
        /root/embedding-project/dvecdb-data

# Checkout old version
cd /root/d-vecDB
git checkout 75bbb9c  # v0.2.0 (last stable before metadata changes)
cargo build --release --bin vectordb-server

# Start old server
sudo systemctl start d-vecdb
```

---

## ✅ Next Steps After Deployment

1. **Monitor for 24 hours:**
   - Watch logs for errors
   - Verify search quality
   - Check memory usage

2. **Test restart behavior:**
   ```bash
   sudo systemctl restart d-vecdb
   # Verify vectors still load correctly
   ```

3. **Update documentation:**
   - Document your migration experience
   - Share any issues encountered

4. **Plan for scale:**
   - If growing beyond 1M vectors, consider sharding
   - Monitor startup time as collection grows

---

**Deployment checklist:**

- [ ] Backup data
- [ ] Pull latest code
- [ ] Rebuild server
- [ ] Choose migration path
- [ ] Verify logs show vectors loaded
- [ ] Test search returns results
- [ ] Test restart behavior
- [ ] Monitor for 24 hours

Good luck! 🚀
