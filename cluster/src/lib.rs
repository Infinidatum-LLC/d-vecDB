pub mod manager;
pub mod node;
pub mod health;
pub mod discovery;
pub mod failover;
pub mod router;
pub mod types;

pub use manager::ClusterManager;
pub use node::{Node, NodeId, NodeRole, NodeInfo};
pub use health::HealthChecker;
pub use discovery::DiscoveryProtocol;
pub use failover::FailoverManager;
pub use router::QueryRouter;
pub use types::*;

use anyhow::Result;

/// Cluster configuration
#[derive(Debug, Clone)]
pub struct ClusterConfig {
    /// This node's ID
    pub node_id: NodeId,

    /// This node's role (Leader, Follower, Observer)
    pub initial_role: NodeRole,

    /// Seed nodes for discovery (bootstrap)
    pub seed_nodes: Vec<String>,

    /// Gossip port for node discovery
    pub gossip_port: u16,

    /// Health check interval (seconds)
    pub health_check_interval: u64,

    /// Election timeout (milliseconds)
    pub election_timeout_ms: u64,

    /// Heartbeat interval (milliseconds)
    pub heartbeat_interval_ms: u64,

    /// Minimum followers for quorum
    pub min_followers_for_quorum: usize,
}

impl Default for ClusterConfig {
    fn default() -> Self {
        Self {
            node_id: NodeId::new(),
            initial_role: NodeRole::Follower,
            seed_nodes: Vec::new(),
            gossip_port: 8090,
            health_check_interval: 5,
            election_timeout_ms: 30000,
            heartbeat_interval_ms: 5000,
            min_followers_for_quorum: 2,
        }
    }
}
