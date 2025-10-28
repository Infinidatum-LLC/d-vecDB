# d-vecDB Multi-Node Distributed Architecture

**Version**: 2.0 (Multi-Node Edition)
**Date**: 2025-10-28
**Status**: 🚧 Implementation In Progress

---

## 🎯 Vision

Transform d-vecDB into a **distributed, multi-node vector database** that maintains **blazing-fast performance** (1.35ms search) while adding:
- ✅ **Horizontal scaling** - Add nodes to increase capacity
- ✅ **High availability** - Automatic failover
- ✅ **Data replication** - Multi-region support
- ✅ **Fault tolerance** - Survive node failures
- ✅ **Load distribution** - Balanced query processing

---

## 🏗️ Architecture Overview

### System Architecture

```
                        ┌─────────────────────────────────────┐
                        │      Load Balancer / Gateway        │
                        │    (Query Router & Coordinator)     │
                        └──────────────┬──────────────────────┘
                                       │
                    ┌──────────────────┼──────────────────┐
                    │                  │                  │
            ┌───────▼───────┐  ┌──────▼──────┐  ┌───────▼───────┐
            │   Node 1      │  │   Node 2    │  │   Node 3      │
            │   (Leader)    │  │  (Follower) │  │  (Follower)   │
            │               │  │             │  │               │
            │ Collections:  │  │ Collections:│  │ Collections:  │
            │  - vectors    │  │  - vectors  │  │  - vectors    │
            │  - embeddings │  │  - embeddings│ │  - embeddings │
            │               │  │             │  │               │
            │ Raft Log      │  │ Raft Log    │  │ Raft Log      │
            │ WAL Storage   │  │ WAL Storage │  │ WAL Storage   │
            └───────┬───────┘  └──────┬──────┘  └───────┬───────┘
                    │                 │                  │
                    └─────────────────┴──────────────────┘
                              Replication Stream
                         (Leader → Followers, Async)
```

---

## 📐 Design Principles

### 1. **CP in CAP Theorem** (Consistency + Partition Tolerance)

We choose **Consistency** over Availability for writes:
- Writes go to leader → replicated → acknowledged
- Reads can go to followers (eventual consistency) or leader (strong consistency)
- Network partitions: Minority partition becomes read-only

**Why**: Vector databases need consistent embeddings. Stale data is better than incorrect data.

### 2. **Performance First**

Maintain sub-2ms search latency:
- ✅ Reads: 95% served by local node (no network hop)
- ✅ HNSW indexes stay in-memory (no serialization)
- ✅ Replication is async (doesn't block queries)
- ✅ Query routing uses smart caching

### 3. **Incremental Rollout**

**Phase 1** (This implementation): Leader-Follower Replication
- 1 Leader (writes + reads)
- N Followers (reads only)
- Async replication
- Manual failover

**Phase 2** (Future): Raft Consensus
- Automatic leader election
- Strong consistency
- Automatic failover
- Multi-leader writes

**Phase 3** (Future): Sharding
- Horizontal data partitioning
- Distributed queries
- Petabyte-scale

---

## 🔧 Architecture Components

### 1. Cluster Manager

**Responsibilities**:
- Node discovery (gossip protocol)
- Health monitoring
- Cluster topology maintenance
- Failover coordination

**Implementation**:
```rust
// cluster/src/manager.rs
pub struct ClusterManager {
    node_id: NodeId,
    role: NodeRole,  // Leader, Follower, Observer
    peers: HashMap<NodeId, PeerInfo>,
    health_checker: HealthChecker,
    config: ClusterConfig,
}

pub enum NodeRole {
    Leader,    // Handles writes, replicates to followers
    Follower,  // Receives replicated data, handles reads
    Observer,  // Monitoring only, no data
}
```

**Discovery Protocol** (Gossip):
```
Node startup:
1. Node announces presence to seed nodes
2. Receives peer list from seed nodes
3. Establishes connections to all peers
4. Starts heartbeat (every 5s)
5. If leader not found → election timeout → vote
```

---

### 2. Replication Engine

**Strategy**: Leader-Follower with Async Replication

**Write Path**:
```
Client → Leader:
  1. Leader receives write request
  2. Leader applies to local VectorStore
  3. Leader appends to ReplicationLog
  4. Leader responds to client (fast!)
  5. Async: Replicate to followers

Replication to Followers:
  1. Leader batches log entries (100ms window)
  2. Sends batch to all followers
  3. Followers apply entries in order
  4. Followers ACK (leader tracks lag)
```

**Read Path**:
```
Client → Any Node:
  1. Query router picks nearest healthy node
  2. Node executes HNSW search (local, fast!)
  3. Returns results

Read Consistency Options:
  - eventual: Any node (fastest, 1.35ms)
  - strong: Leader only (slower, 2-3ms)
```

**Replication Log Format**:
```rust
pub enum ReplicationEntry {
    CreateCollection(CollectionConfig),
    InsertVector {
        collection: String,
        vector: Vector,
        timestamp: u64,
    },
    DeleteVector {
        collection: String,
        vector_id: Uuid,
        timestamp: u64,
    },
    Checkpoint {
        sequence_number: u64,
        timestamp: u64,
    },
}
```

---

### 3. Query Router / Gateway

**Responsibilities**:
- Route reads to optimal node (latency, load)
- Route writes to leader
- Aggregate distributed queries (future: sharding)
- Circuit breaking for unhealthy nodes

**Routing Strategy**:
```rust
impl QueryRouter {
    pub async fn route_read(&self, request: SearchRequest) -> Node {
        // 1. Filter healthy nodes
        let healthy = self.get_healthy_nodes();

        // 2. Prefer local node (same AZ/region)
        if let Some(local) = self.find_local_node(&healthy) {
            return local;
        }

        // 3. Pick least loaded node
        self.pick_least_loaded(&healthy)
    }

    pub async fn route_write(&self, request: WriteRequest) -> Node {
        // Always route to leader
        self.get_leader()
    }
}
```

**Load Balancing**:
- Round-robin for reads
- Least-connections for heavy queries
- Latency-based routing (prefer low-latency nodes)

---

### 4. Failover Manager

**Automatic Leader Election** (Simple majority):
```
Leader failure detected:
  1. Followers detect leader timeout (30s)
  2. Followers enter candidate state
  3. Each candidate votes for self
  4. Candidate requests votes from peers
  5. Majority wins → becomes leader
  6. New leader announces to cluster
  7. Followers reconnect to new leader
  8. Resume normal operation

Follower failure:
  - Leader marks follower as unhealthy
  - Query router stops routing to it
  - Leader keeps trying to reconnect
  - When follower recovers, catch-up replication
```

**Split-Brain Prevention**:
- Require majority (quorum) for leader election
- If network partition: minority partition becomes read-only
- Fencing tokens prevent dual-leader writes

---

### 5. Data Consistency

**Replication Lag Tracking**:
```rust
pub struct ReplicationState {
    last_applied_sequence: u64,
    leader_sequence: u64,
    lag_ms: u64,
}

impl Leader {
    pub fn get_follower_lag(&self, follower_id: NodeId) -> Duration {
        let follower_seq = self.follower_positions.get(follower_id);
        let leader_seq = self.current_sequence;
        let lag = leader_seq - follower_seq;

        // Estimate time lag based on replication rate
        Duration::from_millis(lag * self.avg_replication_latency_ms)
    }
}
```

**Read-Your-Writes Consistency**:
```rust
// Client gets token after write
let token = client.insert(vector).await?;

// Read must be >= token sequence
let results = client.search_with_consistency(
    query,
    ConsistencyLevel::ReadYourWrites(token)
).await?;
```

---

### 6. Network Protocol

**Inter-Node Communication**: gRPC (efficient binary protocol)

```protobuf
service ClusterService {
  // Replication
  rpc ReplicateLog(stream ReplicationEntry) returns (ReplicationResponse);

  // Health & Discovery
  rpc Heartbeat(HeartbeatRequest) returns (HeartbeatResponse);
  rpc JoinCluster(JoinRequest) returns (ClusterTopology);

  // Failover
  rpc RequestVote(VoteRequest) returns (VoteResponse);
  rpc AnnounceLeader(LeaderAnnouncement) returns (Acknowledgment);

  // Data sync
  rpc SyncSnapshot(SnapshotRequest) returns (stream SnapshotChunk);
}
```

---

## 🚀 Performance Optimizations

### 1. Zero-Copy Replication

```rust
// Leader doesn't serialize vectors for replication
// Instead: Send memory-mapped file offsets
pub struct ReplicationEntry {
    operation: Operation,
    data_ref: MMapOffset,  // Point to mmap region
    checksum: u64,
}
```

### 2. Batch Replication

```rust
// Buffer writes for 100ms, then replicate batch
const BATCH_WINDOW: Duration = Duration::from_millis(100);
const MAX_BATCH_SIZE: usize = 1000;

impl ReplicationEngine {
    async fn batch_replicate(&mut self) {
        let mut batch = Vec::new();
        let deadline = Instant::now() + BATCH_WINDOW;

        while Instant::now() < deadline && batch.len() < MAX_BATCH_SIZE {
            if let Some(entry) = self.log_queue.try_recv() {
                batch.push(entry);
            }
        }

        // Send entire batch in one gRPC call
        self.send_batch(batch).await;
    }
}
```

### 3. Parallel Query Execution

```rust
// For distributed queries (future sharding)
async fn distributed_search(&self, query: SearchRequest) -> Vec<QueryResult> {
    let nodes = self.get_shard_nodes(&query.collection);

    // Query all shards in parallel
    let futures: Vec<_> = nodes.iter()
        .map(|node| node.search(query.clone()))
        .collect();

    let results = join_all(futures).await;

    // Merge and re-rank top-k
    self.merge_results(results, query.limit)
}
```

### 4. Local-First Reads

```rust
// Prefer local node (no network latency)
impl QueryRouter {
    fn route_read(&self, collection: &str) -> NodeId {
        if self.local_node.has_collection(collection) {
            return self.local_node.id;
        }

        // Fallback to remote nodes
        self.nearest_node_with_collection(collection)
    }
}
```

**Latency Breakdown**:
```
Single-node search:     1.35ms
  - HNSW traversal:     1.30ms
  - Metadata fetch:     0.05ms

Multi-node search (eventual consistency):  1.40ms
  - Query routing:      0.05ms
  - HNSW traversal:     1.30ms
  - Metadata fetch:     0.05ms

Multi-node search (strong consistency):    2.50ms
  - Leader check:       1.00ms (network RTT)
  - HNSW traversal:     1.30ms
  - Metadata fetch:     0.20ms
```

---

## 📊 Cluster Configurations

### Small Cluster (3 nodes)

```yaml
cluster:
  nodes: 3
  topology: leader-follower

  node_specs:
    cpu: 4 cores
    memory: 8 GB
    storage: 100 GB SSD

  performance:
    write_throughput: 20K vectors/sec (leader)
    read_throughput: 120K queries/sec (3 nodes × 40K)
    search_latency_p50: 1.4ms
    search_latency_p99: 2.8ms

  availability:
    rpo: 1 second (replication lag)
    rto: 30 seconds (failover)
    tolerate: 1 node failure
```

### Medium Cluster (9 nodes)

```yaml
cluster:
  nodes: 9 (1 leader + 8 followers)
  topology: leader-follower

  node_specs:
    cpu: 8 cores
    memory: 16 GB
    storage: 200 GB NVMe

  performance:
    write_throughput: 50K vectors/sec
    read_throughput: 400K queries/sec (9 × 45K)
    search_latency_p50: 1.3ms
    search_latency_p99: 2.5ms

  availability:
    rpo: 500ms
    rto: 15 seconds
    tolerate: 4 node failures (keep quorum)
```

### Large Cluster (27 nodes) - Future: Sharding

```yaml
cluster:
  nodes: 27 (3 shards × 9 replicas)
  topology: sharded + replicated

  shard_strategy: hash(collection_name)

  performance:
    write_throughput: 150K vectors/sec
    read_throughput: 1.2M queries/sec
    search_latency_p50: 1.5ms (distributed)
    search_latency_p99: 3.2ms

  capacity:
    total_storage: 5.4 TB
    total_vectors: 5B+ vectors
```

---

## 🔐 Consistency Guarantees

### Read Consistency Levels

```rust
pub enum ConsistencyLevel {
    /// Read from any node (fastest, eventual consistency)
    /// Latency: ~1.4ms
    /// Guarantee: May see stale data (up to replication lag)
    Eventual,

    /// Read from leader (strong consistency)
    /// Latency: ~2.5ms
    /// Guarantee: Always see latest writes
    Strong,

    /// Read must include client's own writes
    /// Latency: ~1.5-2.5ms (depends on follower lag)
    /// Guarantee: See your own writes
    ReadYourWrites { write_token: u64 },

    /// Read from majority of nodes
    /// Latency: ~3.0ms
    /// Guarantee: Linearizable reads
    Quorum,
}
```

### Write Guarantees

```rust
pub enum WriteAck {
    /// Leader applies, responds immediately (fastest)
    /// Risk: Data loss if leader crashes before replication
    LeaderOnly,

    /// Wait for 1 follower to replicate (default)
    /// Safe: Can tolerate 1 node failure
    OneCopy,

    /// Wait for majority to replicate (safest)
    /// Safe: Can tolerate N/2 failures
    Quorum,
}
```

**Default Configuration**:
- Reads: `Eventual` (1.4ms, 99% of use cases)
- Writes: `OneCopy` (balanced safety & speed)

---

## 🛠️ Implementation Phases

### Phase 1: Basic Replication (Current) ✅

**Timeline**: Week 1-2

**Features**:
- [x] Cluster manager with gossip protocol
- [x] Leader-follower replication
- [x] Async replication log
- [x] Basic failover (manual)
- [x] Query routing

**Deliverables**:
- New crate: `cluster/`
- New crate: `replication/`
- Updated: `server/` for cluster mode
- Tests: 3-node cluster tests

---

### Phase 2: Automatic Failover (Next) 🚧

**Timeline**: Week 3-4

**Features**:
- [ ] Raft consensus for leader election
- [ ] Automatic failover (< 30s)
- [ ] Split-brain prevention
- [ ] Snapshot & log compaction

**Deliverables**:
- Raft implementation
- Failover tests
- Multi-region deployment guide

---

### Phase 3: Sharding (Future) 📋

**Timeline**: Week 5-8

**Features**:
- [ ] Hash-based sharding
- [ ] Distributed query aggregation
- [ ] Rebalancing on scale events
- [ ] Cross-shard transactions

---

## 📈 Performance Benchmarks (Projected)

| Metric | Single Node | 3-Node Cluster | 9-Node Cluster |
|--------|-------------|----------------|----------------|
| **Write Throughput** | 7K/sec | 20K/sec | 50K/sec |
| **Read Throughput** | 13K/sec | 40K/sec | 120K/sec |
| **Search Latency P50** | 1.35ms | 1.40ms | 1.35ms |
| **Search Latency P99** | 2.50ms | 2.80ms | 2.50ms |
| **Availability** | 99% | 99.9% | 99.99% |
| **RPO** | N/A | 1s | 500ms |
| **RTO** | N/A | 30s | 15s |

**Key Insight**: Linear read scaling, minimal latency overhead! 🚀

---

## 🎯 Success Criteria

### Performance
- ✅ Search latency < 2ms (P99)
- ✅ Linear read scaling (N nodes = N× throughput)
- ✅ Write throughput > 20K/sec (3-node)
- ✅ Replication lag < 1 second (P99)

### Reliability
- ✅ Automatic failover < 30s
- ✅ Zero data loss (OneCopy write ack)
- ✅ Survive (N-1)/2 node failures
- ✅ RPO < 1 second

### Scalability
- ✅ Support 3-27 nodes
- ✅ Add nodes without downtime
- ✅ Graceful degradation on failures

---

## 📝 Configuration Example

```toml
# config.toml
[cluster]
enabled = true
node_id = "node-1"
role = "leader"  # or "follower" or "observer"

# Seed nodes for discovery
seed_nodes = [
  "node-1.cluster.local:8090",
  "node-2.cluster.local:8090",
  "node-3.cluster.local:8090"
]

[cluster.gossip]
port = 8090
interval_ms = 5000
timeout_ms = 10000

[cluster.replication]
batch_size = 1000
batch_window_ms = 100
max_lag_seconds = 10

[cluster.failover]
election_timeout_ms = 30000
heartbeat_interval_ms = 5000
vote_timeout_ms = 5000

[cluster.routing]
strategy = "least-loaded"  # or "round-robin" or "latency-based"
local_preference = true
```

---

## 🚀 Next Steps

1. ✅ Implement cluster manager
2. ✅ Implement replication engine
3. ✅ Add query router
4. ✅ Create multi-node tests
5. 🚧 Benchmark 3-node cluster
6. 🚧 Deploy to Kubernetes (multi-node)
7. 📋 Implement Raft consensus
8. 📋 Add sharding support

---

**Status**: 🔥 **IMPLEMENTATION IN PROGRESS**

**Target Release**: v0.2.0 (Multi-Node Edition)

---

Generated with ❤️ for blazing-fast distributed vector search
