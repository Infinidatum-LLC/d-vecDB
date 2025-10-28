use crate::types::*;
use crate::node::Node;
use anyhow::Result;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error};

/// Cluster manager - coordinates the cluster
pub struct ClusterManager {
    /// This node
    pub local_node: Arc<Node>,

    /// All known nodes in the cluster
    pub nodes: Arc<DashMap<NodeId, Arc<Node>>>,

    /// Current cluster topology
    pub topology: Arc<RwLock<ClusterTopology>>,

    /// Current election term
    pub term: Arc<RwLock<u64>>,

    /// Configuration
    pub config: ClusterConfig,
}

impl ClusterManager {
    /// Create a new cluster manager
    pub fn new(config: ClusterConfig) -> Self {
        let local_addr = format!("0.0.0.0:{}", config.gossip_port)
            .parse()
            .expect("Invalid address");

        let local_node = Arc::new(Node::new(
            config.node_id.clone(),
            config.initial_role,
            local_addr,
            8080,  // TODO: Get from server config
            9090,
            config.gossip_port,
        ));

        let nodes = DashMap::new();
        nodes.insert(config.node_id.clone(), local_node.clone());

        Self {
            local_node,
            nodes: Arc::new(nodes),
            topology: Arc::new(RwLock::new(ClusterTopology::new())),
            term: Arc::new(RwLock::new(0)),
            config,
        }
    }

    /// Start the cluster manager
    pub async fn start(self: Arc<Self>) -> Result<()> {
        info!("Starting cluster manager: node_id={}", self.local_node.id);

        // Mark local node as healthy
        self.local_node.set_state(NodeState::Healthy);

        // Start background tasks
        let health_checker = self.clone();
        tokio::spawn(async move {
            health_checker.run_health_checks().await;
        });

        let heartbeat_sender = self.clone();
        tokio::spawn(async move {
            heartbeat_sender.send_heartbeats().await;
        });

        // If not leader, monitor leader health
        if !self.local_node.is_leader() {
            let leader_monitor = self.clone();
            tokio::spawn(async move {
                leader_monitor.monitor_leader().await;
            });
        }

        info!("Cluster manager started");

        Ok(())
    }

    /// Get the current leader node
    pub async fn get_leader(&self) -> Option<Arc<Node>> {
        let topology = self.topology.read().await;

        topology.leader.as_ref()
            .and_then(|leader_id| self.nodes.get(leader_id).map(|n| n.clone()))
    }

    /// Check if this node is the leader
    pub fn is_leader(&self) -> bool {
        self.local_node.is_leader()
    }

    /// Get all healthy nodes
    pub fn get_healthy_nodes(&self) -> Vec<Arc<Node>> {
        self.nodes
            .iter()
            .filter(|entry| entry.value().is_healthy())
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Get all follower nodes
    pub fn get_followers(&self) -> Vec<Arc<Node>> {
        self.nodes
            .iter()
            .filter(|entry| {
                matches!(entry.value().get_role(), NodeRole::Follower)
                    && entry.value().is_healthy()
            })
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Add a node to the cluster
    pub async fn add_node(&self, node: Arc<Node>) -> Result<()> {
        info!("Adding node to cluster: node_id={}, role={}",
              node.id, node.get_role());

        self.nodes.insert(node.id.clone(), node.clone());

        // Update topology
        self.update_topology().await;

        Ok(())
    }

    /// Remove a node from the cluster
    pub async fn remove_node(&self, node_id: &NodeId) -> Result<()> {
        info!("Removing node from cluster: node_id={}", node_id);

        self.nodes.remove(node_id);

        // Update topology
        self.update_topology().await;

        Ok(())
    }

    /// Update cluster topology
    async fn update_topology(&self) {
        let mut topology = self.topology.write().await;

        // Find leader
        topology.leader = self.nodes
            .iter()
            .find(|entry| entry.value().is_leader())
            .map(|entry| entry.key().clone());

        // Get followers
        topology.followers = self.nodes
            .iter()
            .filter(|entry| matches!(entry.value().get_role(), NodeRole::Follower))
            .map(|entry| entry.key().clone())
            .collect();

        // Get observers
        topology.observers = self.nodes
            .iter()
            .filter(|entry| matches!(entry.value().get_role(), NodeRole::Observer))
            .map(|entry| entry.key().clone())
            .collect();

        // Update node info
        topology.nodes = self.nodes
            .iter()
            .map(|entry| entry.value().get_info())
            .collect();

        topology.term = *self.term.read().await;
    }

    /// Run periodic health checks
    async fn run_health_checks(&self) {
        let interval = std::time::Duration::from_secs(self.config.health_check_interval);

        loop {
            tokio::time::sleep(interval).await;

            for entry in self.nodes.iter() {
                let node = entry.value();

                // Skip self
                if node.id == self.local_node.id {
                    continue;
                }

                // TODO: Implement actual health check (ping via gRPC)
                // For now, just log
                tracing::trace!("Health check: node_id={}", node.id);
            }
        }
    }

    /// Send periodic heartbeats (if leader)
    async fn send_heartbeats(&self) {
        if !self.local_node.is_leader() {
            return;
        }

        let interval = std::time::Duration::from_millis(self.config.heartbeat_interval_ms);

        loop {
            tokio::time::sleep(interval).await;

            let term = *self.term.read().await;

            for follower in self.get_followers() {
                // TODO: Send actual heartbeat via gRPC
                tracing::trace!("Sending heartbeat to follower: node_id={}", follower.id);
            }
        }
    }

    /// Monitor leader health (if follower)
    async fn monitor_leader(&self) {
        let timeout = std::time::Duration::from_millis(self.config.election_timeout_ms);

        loop {
            tokio::time::sleep(timeout).await;

            // Check if leader is healthy
            if let Some(leader) = self.get_leader().await {
                if !leader.is_healthy() {
                    warn!("Leader unhealthy, may need to start election");
                    // TODO: Trigger election
                }
            } else {
                warn!("No leader found, may need to start election");
                // TODO: Trigger election
            }
        }
    }

    /// Get cluster statistics
    pub async fn get_stats(&self) -> ClusterStats {
        let topology = self.topology.read().await;

        ClusterStats {
            total_nodes: self.nodes.len(),
            healthy_nodes: self.get_healthy_nodes().len(),
            leader: topology.leader.clone(),
            followers_count: topology.followers.len(),
            observers_count: topology.observers.len(),
            term: topology.term,
        }
    }
}

/// Cluster statistics
#[derive(Debug, Clone)]
pub struct ClusterStats {
    pub total_nodes: usize,
    pub healthy_nodes: usize,
    pub leader: Option<NodeId>,
    pub followers_count: usize,
    pub observers_count: usize,
    pub term: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cluster_manager_creation() {
        let config = ClusterConfig::default();
        let manager = Arc::new(ClusterManager::new(config));

        assert_eq!(manager.nodes.len(), 1); // Only local node
        assert!(!manager.is_leader());
    }

    #[tokio::test]
    async fn test_add_remove_node() {
        let config = ClusterConfig::default();
        let manager = Arc::new(ClusterManager::new(config));

        // Add a follower node
        let follower_node = Arc::new(Node::new(
            NodeId::new(),
            NodeRole::Follower,
            "127.0.0.1:8091".parse().unwrap(),
            8081,
            9091,
            8091,
        ));

        manager.add_node(follower_node.clone()).await.unwrap();

        assert_eq!(manager.nodes.len(), 2);

        // Remove the node
        manager.remove_node(&follower_node.id).await.unwrap();

        assert_eq!(manager.nodes.len(), 1);
    }
}
