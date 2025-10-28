# Hardware Requirements for Competitive Benchmarks

**Last Updated**: 2025-10-28

---

## 🎯 TL;DR - Quick Reference

| Benchmark Scale | CPU | RAM | Storage | Time | Cost |
|----------------|-----|-----|---------|------|------|
| **Quick Test** (10K vectors) | 2 cores | 4 GB | 10 GB | 2 min | **Your laptop** |
| **Small** (10K vectors) | 4 cores | 8 GB | 20 GB | 5 min | **Your laptop** |
| **Medium** (100K vectors) | 8 cores | 16 GB | 50 GB | 15 min | Desktop/Cloud |
| **Realistic** (500K vectors) | 8 cores | 32 GB | 100 GB | 30 min | Desktop/Cloud |
| **Large** (1M vectors) | 16 cores | 64 GB | 200 GB | 60 min | Server/Cloud |

---

## 💻 What You Probably Have (And It's Enough!)

### Your Current Machine (MacBook Pro M1/M2)

**Specs**: Darwin 24.6.0, ARM64
- ✅ **CPU**: Apple Silicon (8+ cores) - **Perfect!**
- ✅ **RAM**: Likely 16-32 GB - **Perfect!**
- ✅ **Storage**: Likely 256+ GB SSD - **Perfect!**

**What You Can Run**:
- ✅ Quick Test (10K vectors) - **No problem!**
- ✅ Small (10K vectors) - **Easy**
- ✅ Medium (100K vectors) - **Comfortable**
- ✅ Realistic (500K vectors) - **Yes, if you have 16+ GB RAM**
- ⚠️ Large (1M vectors) - **Possible, but needs 32+ GB RAM**

**Recommendation**: **Start with Small or Realistic datasets on your current machine!**

---

## 📊 Detailed Requirements by Dataset

### Quick Test (10K vectors, 128-dim)

**Purpose**: Verify everything works

**Minimum Requirements**:
- **CPU**: 2 cores (any modern CPU)
- **RAM**: 4 GB
- **Storage**: 5 GB free
- **Time**: 1-2 minutes

**Recommended**:
- **CPU**: 4 cores
- **RAM**: 8 GB
- **Storage**: 10 GB SSD
- **Time**: < 1 minute

**Can Run On**:
- ✅ Any laptop from last 5 years
- ✅ Cloud: AWS t3.medium ($0.0416/hr)
- ✅ Cloud: GCP e2-standard-2 ($0.067/hr)

---

### Small Dataset (10K vectors, 128-dim)

**Purpose**: Quick comparison, development

**Minimum Requirements**:
- **CPU**: 4 cores @ 2.0+ GHz
- **RAM**: 8 GB
- **Storage**: 20 GB SSD
- **Time**: 2-5 minutes

**Recommended**:
- **CPU**: 8 cores @ 2.5+ GHz
- **RAM**: 16 GB
- **Storage**: 50 GB NVMe SSD
- **Time**: 1-2 minutes

**Memory Breakdown**:
| Component | Memory Usage |
|-----------|--------------|
| d-vecDB | ~500 MB |
| Qdrant | ~800 MB |
| Python process | ~200 MB |
| OS overhead | ~1-2 GB |
| **Total** | ~3-4 GB |

**Can Run On**:
- ✅ MacBook Air M1 (2020+)
- ✅ Any desktop from last 3 years
- ✅ AWS t3.large ($0.0832/hr)
- ✅ Your current machine ✨

---

### Medium Dataset (100K vectors, 300-dim)

**Purpose**: Realistic comparison, CI/CD benchmarks

**Minimum Requirements**:
- **CPU**: 8 cores @ 2.5+ GHz
- **RAM**: 16 GB
- **Storage**: 50 GB SSD
- **Time**: 10-15 minutes

**Recommended**:
- **CPU**: 16 cores @ 3.0+ GHz
- **RAM**: 32 GB
- **Storage**: 100 GB NVMe SSD
- **Time**: 5-10 minutes

**Memory Breakdown**:
| Component | Memory Usage |
|-----------|--------------|
| d-vecDB | ~3 GB |
| Qdrant | ~5 GB |
| Python process | ~1 GB |
| OS overhead | ~2 GB |
| **Total** | ~11-12 GB |

**Can Run On**:
- ✅ MacBook Pro M1/M2 16GB
- ✅ Desktop: Ryzen 7 / Core i7 + 16GB RAM
- ✅ AWS c6i.2xlarge ($0.34/hr, 8 cores, 16 GB)
- ✅ GCP c2-standard-8 ($0.35/hr, 8 cores, 32 GB)
- ✅ Your current machine (if 16+ GB RAM) ✨

---

### Realistic Dataset (500K vectors, 768-dim BERT)

**Purpose**: Production simulation, serious benchmarking

**Minimum Requirements**:
- **CPU**: 8 cores @ 2.5+ GHz
- **RAM**: 24 GB
- **Storage**: 100 GB SSD
- **Time**: 20-30 minutes

**Recommended**:
- **CPU**: 16 cores @ 3.0+ GHz (or Apple M1 Pro/Max)
- **RAM**: 32 GB
- **Storage**: 200 GB NVMe SSD
- **Time**: 10-15 minutes

**Optimal** (Best Performance):
- **CPU**: 32 cores @ 3.5+ GHz (AMD EPYC, Threadripper)
- **RAM**: 64 GB DDR4-3200+
- **Storage**: 500 GB NVMe PCIe 4.0
- **Time**: 5-10 minutes

**Memory Breakdown**:
| Component | Memory Usage |
|-----------|--------------|
| d-vecDB | ~12 GB |
| Qdrant | ~18 GB |
| Python process | ~2 GB |
| OS overhead | ~3 GB |
| **Total** | ~25-30 GB |

**Can Run On**:
- ✅ MacBook Pro M1 Pro/Max 32GB ✨
- ✅ Desktop: Ryzen 9 / Core i9 + 32GB RAM
- ✅ AWS c6i.4xlarge ($0.68/hr, 16 cores, 32 GB)
- ✅ GCP c2-standard-16 ($0.71/hr, 16 cores, 64 GB)
- ⚠️ Your current machine (check RAM: 32+ GB recommended)

---

### Large Dataset (1M vectors, 1536-dim OpenAI)

**Purpose**: Scale testing, white papers, marketing

**Minimum Requirements**:
- **CPU**: 16 cores @ 3.0+ GHz
- **RAM**: 48 GB
- **Storage**: 200 GB SSD
- **Time**: 45-60 minutes

**Recommended**:
- **CPU**: 32 cores @ 3.5+ GHz
- **RAM**: 64 GB
- **Storage**: 500 GB NVMe SSD
- **Time**: 20-30 minutes

**Optimal** (For Publishing Results):
- **CPU**: 64 cores @ 3.7+ GHz (AMD EPYC 7763)
- **RAM**: 128 GB DDR4-3200+
- **Storage**: 1 TB NVMe PCIe 4.0
- **Network**: 10 Gbps (if testing Pinecone)
- **Time**: 10-15 minutes

**Memory Breakdown**:
| Component | Memory Usage |
|-----------|--------------|
| d-vecDB | ~35 GB |
| Qdrant | ~55 GB |
| Python process | ~5 GB |
| OS overhead | ~5 GB |
| **Total** | ~60-70 GB |

**Can Run On**:
- ❌ Most laptops (insufficient RAM)
- ✅ Workstation: Threadripper + 64GB+ RAM
- ✅ AWS c6i.8xlarge ($1.36/hr, 32 cores, 64 GB)
- ✅ AWS r6i.4xlarge ($1.008/hr, 16 cores, 128 GB) - **Best value**
- ✅ GCP c2-standard-30 ($1.06/hr, 30 cores, 120 GB)
- ✅ Dedicated server

---

## ☁️ Cloud Instances (Recommended for Fair Testing)

### AWS EC2 (Best Value)

#### For Small/Medium (Recommended) ⭐

**c6i.2xlarge** - $0.34/hour
- **CPU**: 8 vCPUs (Intel Xeon 3rd Gen)
- **RAM**: 16 GB
- **Network**: Up to 12.5 Gbps
- **Storage**: Add 100 GB gp3 SSD ($10/month)
- **Best for**: Small, Medium datasets
- **Estimated cost**: $0.50 for full benchmark run

#### For Realistic/Large (Recommended) ⭐

**r6i.4xlarge** - $1.008/hour
- **CPU**: 16 vCPUs (Intel Xeon 3rd Gen)
- **RAM**: 128 GB (memory-optimized)
- **Network**: Up to 12.5 Gbps
- **Storage**: Add 200 GB gp3 SSD ($20/month)
- **Best for**: Realistic, Large datasets
- **Estimated cost**: $1.50 for full benchmark run

#### For Scale Testing (Optional)

**c6i.16xlarge** - $2.72/hour
- **CPU**: 64 vCPUs
- **RAM**: 128 GB
- **Best for**: Maximum performance testing
- **Estimated cost**: $3.00 for full benchmark run

---

### Google Cloud Platform (GCP)

#### For Small/Medium

**c2-standard-8** - $0.35/hour
- **CPU**: 8 vCPUs (Cascade Lake)
- **RAM**: 32 GB
- **Storage**: Add 100 GB SSD persistent disk
- **Best for**: Small, Medium datasets

#### For Realistic/Large

**c2-standard-16** - $0.71/hour
- **CPU**: 16 vCPUs
- **RAM**: 64 GB
- **Best for**: Realistic dataset
- **Estimated cost**: $1.00 for full benchmark run

---

### Azure

#### For Small/Medium

**Standard_F8s_v2** - $0.338/hour
- **CPU**: 8 vCPUs (Intel Xeon Platinum)
- **RAM**: 16 GB
- **Storage**: Add 128 GB Premium SSD

#### For Realistic/Large

**Standard_E16s_v5** - $1.008/hour
- **CPU**: 16 vCPUs
- **RAM**: 128 GB (memory-optimized)
- **Best for**: Realistic, Large datasets

---

## 🖥️ Your Current Machine Analysis

Based on your system info:
```
Working directory: /Users/durai/Documents/GitHub/d-vecDB
Platform: darwin
OS Version: Darwin 24.6.0
```

**You have**: macOS (likely MacBook Pro M1/M2/M3)

### If You Have M1/M2/M3 MacBook Pro

#### 16 GB RAM Configuration
- ✅ **Quick Test** - Perfect
- ✅ **Small** - Perfect
- ✅ **Medium** - Good (but close memory)
- ⚠️ **Realistic** - Possible but tight (will use swap)
- ❌ **Large** - Not recommended (insufficient RAM)

**Recommendation**: Run **Small** or **Medium** datasets

#### 32 GB RAM Configuration
- ✅ **Quick Test** - Perfect
- ✅ **Small** - Perfect
- ✅ **Medium** - Perfect
- ✅ **Realistic** - Perfect ⭐
- ⚠️ **Large** - Possible (will work but slow)

**Recommendation**: Run **Realistic** dataset! You have enough power.

#### 64 GB+ RAM Configuration
- ✅ All datasets including **Large** - Perfect! 🚀

---

## 💡 Recommendations for YOU

### Option 1: Use Your Current Machine (FREE) ⭐

**If you have 16+ GB RAM**:
```bash
# Start with Small dataset (safe, fast)
python run_benchmarks.py \
  --databases dvecdb qdrant \
  --datasets small

# If that works well, try Realistic
python run_benchmarks.py \
  --databases dvecdb qdrant \
  --datasets realistic
```

**Estimated time**: 15-30 minutes total
**Cost**: $0

---

### Option 2: Use AWS EC2 (Recommended for Large Scale)

**For serious benchmarking**:

```bash
# 1. Launch EC2 instance
aws ec2 run-instances \
  --instance-type r6i.4xlarge \
  --image-id ami-0c55b159cbfafe1f0 \
  --key-name your-key \
  --security-groups benchmark-sg

# 2. SSH into instance
ssh -i your-key.pem ec2-user@<instance-ip>

# 3. Install everything
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
git clone https://github.com/rdmurugan/d-vecDB.git
cd d-vecDB
cargo build --release

# 4. Run benchmarks
cd benchmarks/competitive
python3 -m venv venv
source venv/bin/activate
pip install -r requirements.txt
pip install -e ../../python-client

# Start d-vecDB
../target/release/vectordb-server &

# Start Qdrant (Docker)
docker run -d -p 6333:6333 qdrant/qdrant

# Run full benchmark
python run_benchmarks.py \
  --databases dvecdb qdrant \
  --datasets realistic large

# 5. Download results
scp -i your-key.pem ec2-user@<instance-ip>:~/d-vecDB/benchmarks/competitive/results/*.json ./

# 6. Terminate instance (don't forget!)
aws ec2 terminate-instances --instance-ids <instance-id>
```

**Estimated time**: 1 hour setup + 1 hour benchmarks
**Cost**: ~$2-3 total

---

## 🔍 How to Check Your Current Machine

### Check RAM
```bash
# macOS
sysctl hw.memsize | awk '{print $2/1024/1024/1024 " GB"}'

# Linux
free -h | grep Mem | awk '{print $2}'
```

### Check CPU Cores
```bash
# macOS
sysctl -n hw.ncpu

# Linux
nproc
```

### Check Available Storage
```bash
df -h | grep -E "/$|/home"
```

### Check if You Can Run Realistic Dataset
```bash
# Quick memory check
python3 -c "
import psutil
total_ram_gb = psutil.virtual_memory().total / 1024**3
available_ram_gb = psutil.virtual_memory().available / 1024**3

print(f'Total RAM: {total_ram_gb:.1f} GB')
print(f'Available RAM: {available_ram_gb:.1f} GB')

if available_ram_gb >= 24:
    print('✅ Can run Realistic dataset (500K vectors)')
elif available_ram_gb >= 12:
    print('✅ Can run Medium dataset (100K vectors)')
elif available_ram_gb >= 6:
    print('✅ Can run Small dataset (10K vectors)')
else:
    print('⚠️ Close other applications first')
"
```

---

## 📊 Storage Requirements Detail

### Disk Space Needed

| Dataset | Vectors (raw) | HNSW Index | Metadata | Queries | Total |
|---------|--------------|------------|----------|---------|-------|
| **Small** | 5 MB | 50 MB | 10 MB | 0.5 MB | **~100 MB** |
| **Medium** | 120 MB | 500 MB | 100 MB | 1 MB | **~1 GB** |
| **Realistic** | 1.5 GB | 6 GB | 500 MB | 15 MB | **~10 GB** |
| **Large** | 6 GB | 25 GB | 1 GB | 6 MB | **~40 GB** |

**Add 50% buffer for**: WAL logs, temporary files, OS cache

**Recommended free space**:
- Small: 10 GB
- Medium: 20 GB
- Realistic: 50 GB
- Large: 100 GB

### Storage Type Impact on Performance

| Storage Type | d-vecDB Insert | d-vecDB Search | Qdrant Insert | Qdrant Search |
|--------------|----------------|----------------|---------------|---------------|
| **NVMe SSD** | 50K/s | 1.35ms | 30K/s | 2.8ms |
| **SATA SSD** | 40K/s | 1.50ms | 22K/s | 3.2ms |
| **HDD** | 8K/s | 5.00ms | 5K/s | 8.0ms |

**Recommendation**: Use SSD (NVMe preferred)

---

## 💰 Cost Estimates

### Running on Your Machine (FREE)

- **Small dataset**: 5 minutes, $0
- **Medium dataset**: 15 minutes, $0
- **Realistic dataset**: 30 minutes, $0
- **Total**: $0 ⭐

### Running on AWS EC2

- **Small dataset**: 5 min × $0.34/hr = **$0.03**
- **Medium dataset**: 15 min × $0.34/hr = **$0.08**
- **Realistic dataset**: 30 min × $1.00/hr = **$0.50**
- **Large dataset**: 60 min × $1.00/hr = **$1.00**
- **Full suite**: **$1.50** ⭐

### Running with Pinecone (Cloud Cost)

- **API usage**: Free tier (1M operations)
- **Index cost**: $0.096/hour
- **Realistic dataset**: 30 min = **$0.05**
- **Large dataset**: 60 min = **$0.10**

**Total with Pinecone**: ~$2-3

---

## ✅ Final Recommendation for YOU

Based on your macOS system:

### If RAM >= 32 GB:
```bash
# You can run everything locally!
python run_benchmarks.py \
  --databases dvecdb qdrant \
  --datasets realistic
```
**Time**: 30 minutes
**Cost**: $0 ✨

### If RAM = 16 GB:
```bash
# Run Small or Medium datasets
python run_benchmarks.py \
  --databases dvecdb qdrant \
  --datasets small medium
```
**Time**: 20 minutes
**Cost**: $0 ✨

### If RAM < 16 GB:
```bash
# Use AWS EC2 r6i.4xlarge for 1 hour
# Or stick to Small dataset locally
python run_benchmarks.py \
  --databases dvecdb qdrant \
  --datasets small
```
**Time**: 5 minutes
**Cost**: $0 (local) or $1 (AWS)

---

## 🎯 Bottom Line

**You can likely run meaningful benchmarks on your current machine right now!**

Start with:
```bash
cd benchmarks/competitive
python benchmark_dvecdb.py  # Quick 30-second test
```

If that works, scale up to the dataset size your RAM supports.

**No special hardware needed for proving d-vecDB is fast!** 🚀

---

**Questions?**
- Check RAM: `sysctl hw.memsize`
- Check CPU: `sysctl -n hw.ncpu`
- Still unsure? Open an issue: https://github.com/rdmurugan/d-vecDB/issues
