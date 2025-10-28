use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use uuid::Uuid;

/// Cluster configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterConfig {
    pub node_id: NodeId,
    pub initial_role: NodeRole,
    pub gossip_port: u16,
    pub heartbeat_interval_ms: u64,
    pub election_timeout_ms: u64,
    pub health_check_interval: u64,
}

impl Default for ClusterConfig {
    fn default() -> Self {
        Self {
            node_id: NodeId::new(),
            initial_role: NodeRole::Follower,
            gossip_port: 7946,
            heartbeat_interval_ms: 1000,
            election_timeout_ms: 5000,
            health_check_interval: 30,
        }
    }
}

/// Unique identifier for a node in the cluster
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub Uuid);

impl NodeId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_string(s: &str) -> Result<Self, uuid::Error> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}

/// Node role in the cluster
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeRole {
    /// Leader: handles writes and replicates to followers
    Leader,

    /// Follower: receives replicated data, handles reads
    Follower,

    /// Candidate: participating in leader election
    Candidate,

    /// Observer: monitoring only, no voting rights
    Observer,
}

impl std::fmt::Display for NodeRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeRole::Leader => write!(f, "Leader"),
            NodeRole::Follower => write!(f, "Follower"),
            NodeRole::Candidate => write!(f, "Candidate"),
            NodeRole::Observer => write!(f, "Observer"),
        }
    }
}

/// Node state in the cluster
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeState {
    /// Node is starting up
    Starting,

    /// Node is healthy and operational
    Healthy,

    /// Node is experiencing issues but still operational
    Degraded,

    /// Node is unhealthy and should not receive traffic
    Unhealthy,

    /// Node is shutting down gracefully
    ShuttingDown,

    /// Node is disconnected from cluster
    Disconnected,
}

/// Information about a node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub id: NodeId,
    pub role: NodeRole,
    pub state: NodeState,
    pub address: SocketAddr,
    pub rest_port: u16,
    pub grpc_port: u16,
    pub gossip_port: u16,
    pub region: Option<String>,
    pub zone: Option<String>,
    pub version: String,
    pub uptime_seconds: u64,
    pub last_heartbeat: u64,
}

/// Cluster topology - all known nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterTopology {
    pub leader: Option<NodeId>,
    pub followers: Vec<NodeId>,
    pub observers: Vec<NodeId>,
    pub nodes: Vec<NodeInfo>,
    pub term: u64,  // Election term (for Raft)
}

impl ClusterTopology {
    pub fn new() -> Self {
        Self {
            leader: None,
            followers: Vec::new(),
            observers: Vec::new(),
            nodes: Vec::new(),
            term: 0,
        }
    }

    pub fn get_node(&self, node_id: &NodeId) -> Option<&NodeInfo> {
        self.nodes.iter().find(|n| &n.id == node_id)
    }

    pub fn get_healthy_nodes(&self) -> Vec<&NodeInfo> {
        self.nodes
            .iter()
            .filter(|n| n.state == NodeState::Healthy)
            .collect()
    }

    pub fn get_leader_info(&self) -> Option<&NodeInfo> {
        self.leader.as_ref().and_then(|id| self.get_node(id))
    }
}

impl Default for ClusterTopology {
    fn default() -> Self {
        Self::new()
    }
}

/// Health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub node_id: NodeId,
    pub state: NodeState,
    pub timestamp: u64,
    pub latency_ms: u64,
    pub details: Option<String>,
}

/// Replication state for a node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationState {
    /// Last sequence number applied to this node
    pub last_applied_sequence: u64,

    /// Last sequence number acknowledged by this node
    pub last_ack_sequence: u64,

    /// Current replication lag (milliseconds)
    pub lag_ms: u64,

    /// Timestamp of last replication
    pub last_replication_ts: u64,
}

impl ReplicationState {
    pub fn new() -> Self {
        Self {
            last_applied_sequence: 0,
            last_ack_sequence: 0,
            lag_ms: 0,
            last_replication_ts: 0,
        }
    }
}

impl Default for ReplicationState {
    fn default() -> Self {
        Self::new()
    }
}

/// Vote request (for leader election)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteRequest {
    pub term: u64,
    pub candidate_id: NodeId,
    pub last_log_index: u64,
    pub last_log_term: u64,
}

/// Vote response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteResponse {
    pub term: u64,
    pub vote_granted: bool,
    pub voter_id: NodeId,
}

/// Heartbeat message (leader → followers)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Heartbeat {
    pub leader_id: NodeId,
    pub term: u64,
    pub leader_commit: u64,
    pub timestamp: u64,
}

/// Heartbeat response (followers → leader)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatResponse {
    pub follower_id: NodeId,
    pub success: bool,
    pub last_applied: u64,
    pub timestamp: u64,
}
