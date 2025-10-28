use crate::types::*;
use anyhow::Result;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use parking_lot::RwLock;

pub use crate::types::{NodeId, NodeRole, NodeInfo};

/// A node in the cluster
#[derive(Debug, Clone)]
pub struct Node {
    pub id: NodeId,
    pub role: Arc<RwLock<NodeRole>>,
    pub state: Arc<RwLock<NodeState>>,
    pub address: SocketAddr,
    pub rest_port: u16,
    pub grpc_port: u16,
    pub gossip_port: u16,
    pub region: Option<String>,
    pub zone: Option<String>,
    pub replication_state: Arc<RwLock<ReplicationState>>,
    start_time: SystemTime,
}

impl Node {
    pub fn new(
        id: NodeId,
        role: NodeRole,
        address: SocketAddr,
        rest_port: u16,
        grpc_port: u16,
        gossip_port: u16,
    ) -> Self {
        Self {
            id,
            role: Arc::new(RwLock::new(role)),
            state: Arc::new(RwLock::new(NodeState::Starting)),
            address,
            rest_port,
            grpc_port,
            gossip_port,
            region: None,
            zone: None,
            replication_state: Arc::new(RwLock::new(ReplicationState::new())),
            start_time: SystemTime::now(),
        }
    }

    /// Get current role
    pub fn get_role(&self) -> NodeRole {
        *self.role.read()
    }

    /// Set role
    pub fn set_role(&self, role: NodeRole) {
        *self.role.write() = role;
    }

    /// Get current state
    pub fn get_state(&self) -> NodeState {
        *self.state.read()
    }

    /// Set state
    pub fn set_state(&self, state: NodeState) {
        *self.state.write() = state;
    }

    /// Check if node is healthy
    pub fn is_healthy(&self) -> bool {
        matches!(self.get_state(), NodeState::Healthy)
    }

    /// Check if this node is the leader
    pub fn is_leader(&self) -> bool {
        matches!(self.get_role(), NodeRole::Leader)
    }

    /// Get uptime in seconds
    pub fn uptime_seconds(&self) -> u64 {
        SystemTime::now()
            .duration_since(self.start_time)
            .unwrap_or_default()
            .as_secs()
    }

    /// Get node info (for cluster topology)
    pub fn get_info(&self) -> NodeInfo {
        NodeInfo {
            id: self.id.clone(),
            role: self.get_role(),
            state: self.get_state(),
            address: self.address,
            rest_port: self.rest_port,
            grpc_port: self.grpc_port,
            gossip_port: self.gossip_port,
            region: self.region.clone(),
            zone: self.zone.clone(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime_seconds: self.uptime_seconds(),
            last_heartbeat: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }

    /// Update replication state
    pub fn update_replication(&self, last_applied: u64, lag_ms: u64) {
        let mut state = self.replication_state.write();
        state.last_applied_sequence = last_applied;
        state.lag_ms = lag_ms;
        state.last_replication_ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }

    /// Get replication lag
    pub fn get_replication_lag(&self) -> u64 {
        self.replication_state.read().lag_ms
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_creation() {
        let node_id = NodeId::new();
        let addr = "127.0.0.1:8080".parse().unwrap();

        let node = Node::new(node_id.clone(), NodeRole::Follower, addr, 8080, 9090, 8090);

        assert_eq!(node.id, node_id);
        assert_eq!(node.get_role(), NodeRole::Follower);
        assert_eq!(node.get_state(), NodeState::Starting);
        assert!(!node.is_healthy());
        assert!(!node.is_leader());
    }

    #[test]
    fn test_node_state_transitions() {
        let node = Node::new(
            NodeId::new(),
            NodeRole::Follower,
            "127.0.0.1:8080".parse().unwrap(),
            8080,
            9090,
            8090,
        );

        node.set_state(NodeState::Healthy);
        assert!(node.is_healthy());

        node.set_role(NodeRole::Leader);
        assert!(node.is_leader());
    }
}
