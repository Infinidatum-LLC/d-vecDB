use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

/// Sharding configuration for a collection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardingConfig {
    /// Number of shards
    pub shard_count: usize,
    /// Sharding method
    pub method: ShardingMethod,
    /// Replication factor (copies of each shard)
    pub replication_factor: usize,
}

/// Sharding methods
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShardingMethod {
    /// Hash-based sharding (default)
    Hash,
    /// Custom shard key from payload
    Custom,
    /// Auto-sharding based on load
    Auto,
}

/// Shard information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shard {
    pub id: usize,
    pub node_id: String,
    pub state: ShardState,
    pub vector_count: usize,
    pub is_active: bool,
}

/// Shard states
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ShardState {
    Active,
    Initializing,
    Migrating,
    Dead,
}

/// Shard router - determines which shard a vector belongs to
pub struct ShardRouter {
    config: ShardingConfig,
    shard_map: HashMap<usize, Vec<String>>, // shard_id -> node_ids
}

impl ShardRouter {
    /// Create a new shard router
    pub fn new(config: ShardingConfig, node_ids: Vec<String>) -> Self {
        let mut shard_map = HashMap::new();

        // Distribute shards across nodes with replication
        for shard_id in 0..config.shard_count {
            let mut nodes = Vec::new();

            for replica in 0..config.replication_factor {
                let node_idx = (shard_id + replica) % node_ids.len();
                nodes.push(node_ids[node_idx].clone());
            }

            shard_map.insert(shard_id, nodes);
        }

        Self { config, shard_map }
    }

    /// Determine shard ID for a vector
    pub fn get_shard_id(&self, vector_id: &Uuid, shard_key: Option<&str>) -> usize {
        match self.config.method {
            ShardingMethod::Hash => {
                // Hash-based sharding using vector ID
                let mut hasher = DefaultHasher::new();
                vector_id.hash(&mut hasher);
                (hasher.finish() as usize) % self.config.shard_count
            }
            ShardingMethod::Custom => {
                // Custom sharding using payload field
                if let Some(key) = shard_key {
                    let mut hasher = DefaultHasher::new();
                    key.hash(&mut hasher);
                    (hasher.finish() as usize) % self.config.shard_count
                } else {
                    // Fall back to hash-based if no key provided
                    self.get_shard_id(vector_id, None)
                }
            }
            ShardingMethod::Auto => {
                // Auto-sharding based on load (simplified)
                // TODO: Implement proper load-based sharding
                self.get_shard_id(vector_id, None)
            }
        }
    }

    /// Get nodes responsible for a shard
    pub fn get_shard_nodes(&self, shard_id: usize) -> Option<&Vec<String>> {
        self.shard_map.get(&shard_id)
    }

    /// Get primary node for a shard (first in replica list)
    pub fn get_primary_node(&self, shard_id: usize) -> Option<&String> {
        self.shard_map.get(&shard_id)?.first()
    }

    /// Get all replica nodes for a shard
    pub fn get_replica_nodes(&self, shard_id: usize) -> Option<Vec<String>> {
        self.shard_map.get(&shard_id).map(|nodes| nodes.clone())
    }

    /// Rebalance shards across nodes
    pub fn rebalance(&mut self, node_ids: Vec<String>) {
        let mut new_shard_map = HashMap::new();

        for shard_id in 0..self.config.shard_count {
            let mut nodes = Vec::new();

            for replica in 0..self.config.replication_factor {
                let node_idx = (shard_id + replica) % node_ids.len();
                nodes.push(node_ids[node_idx].clone());
            }

            new_shard_map.insert(shard_id, nodes);
        }

        self.shard_map = new_shard_map;
    }

    /// Check if shard should be migrated
    pub fn needs_migration(&self, shard_id: usize, current_node: &str) -> bool {
        if let Some(nodes) = self.shard_map.get(&shard_id) {
            !nodes.contains(&current_node.to_string())
        } else {
            false
        }
    }
}

/// Consistent hashing ring for better shard distribution
pub struct ConsistentHashRing {
    virtual_nodes: Vec<(u64, String)>, // (hash, node_id)
    vnodes_per_node: usize,
}

impl ConsistentHashRing {
    /// Create a new consistent hash ring
    pub fn new(nodes: Vec<String>, vnodes_per_node: usize) -> Self {
        let mut virtual_nodes = Vec::new();

        for node in nodes {
            for i in 0..vnodes_per_node {
                let vnode_key = format!("{}:{}", node, i);
                let hash = Self::hash_key(&vnode_key);
                virtual_nodes.push((hash, node.clone()));
            }
        }

        virtual_nodes.sort_by_key(|&(hash, _)| hash);

        Self {
            virtual_nodes,
            vnodes_per_node,
        }
    }

    /// Find node for a key using consistent hashing
    pub fn get_node(&self, key: &str) -> Option<&String> {
        if self.virtual_nodes.is_empty() {
            return None;
        }

        let hash = Self::hash_key(key);

        // Binary search for the first virtual node >= hash
        let idx = match self
            .virtual_nodes
            .binary_search_by_key(&hash, |&(h, _)| h)
        {
            Ok(i) => i,
            Err(i) => i % self.virtual_nodes.len(),
        };

        Some(&self.virtual_nodes[idx].1)
    }

    /// Get N nodes for a key (for replication)
    pub fn get_nodes(&self, key: &str, count: usize) -> Vec<String> {
        if self.virtual_nodes.is_empty() {
            return Vec::new();
        }

        let hash = Self::hash_key(key);

        let start_idx = match self
            .virtual_nodes
            .binary_search_by_key(&hash, |&(h, _)| h)
        {
            Ok(i) => i,
            Err(i) => i % self.virtual_nodes.len(),
        };

        let mut nodes = Vec::new();
        let mut seen = std::collections::HashSet::new();
        let mut idx = start_idx;

        while nodes.len() < count && seen.len() < self.virtual_nodes.len() {
            let node = &self.virtual_nodes[idx].1;
            if seen.insert(node.clone()) {
                nodes.push(node.clone());
            }
            idx = (idx + 1) % self.virtual_nodes.len();
        }

        nodes
    }

    fn hash_key(key: &str) -> u64 {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish()
    }
}

/// Shard migration task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardMigration {
    pub shard_id: usize,
    pub from_node: String,
    pub to_node: String,
    pub state: MigrationState,
    pub progress: f32,
    pub started_at: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MigrationState {
    Pending,
    InProgress,
    Completed,
    Failed,
}

/// Shard manager coordinates shard operations
pub struct ShardManager {
    collections: HashMap<String, ShardRouter>,
    migrations: Vec<ShardMigration>,
}

impl ShardManager {
    pub fn new() -> Self {
        Self {
            collections: HashMap::new(),
            migrations: Vec::new(),
        }
    }

    /// Add collection with sharding
    pub fn add_collection(
        &mut self,
        collection: String,
        config: ShardingConfig,
        nodes: Vec<String>,
    ) {
        let router = ShardRouter::new(config, nodes);
        self.collections.insert(collection, router);
    }

    /// Get shard router for collection
    pub fn get_router(&self, collection: &str) -> Option<&ShardRouter> {
        self.collections.get(collection)
    }

    /// Start shard migration
    pub fn start_migration(&mut self, migration: ShardMigration) {
        self.migrations.push(migration);
    }

    /// Get active migrations
    pub fn get_active_migrations(&self) -> Vec<&ShardMigration> {
        self.migrations
            .iter()
            .filter(|m| m.state == MigrationState::InProgress)
            .collect()
    }

    /// Update migration progress
    pub fn update_migration(&mut self, shard_id: usize, progress: f32, state: MigrationState) {
        if let Some(migration) = self.migrations.iter_mut().find(|m| m.shard_id == shard_id) {
            migration.progress = progress;
            migration.state = state;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shard_routing() {
        let config = ShardingConfig {
            shard_count: 4,
            method: ShardingMethod::Hash,
            replication_factor: 2,
        };

        let nodes = vec!["node1".to_string(), "node2".to_string()];
        let router = ShardRouter::new(config, nodes);

        let vector_id = Uuid::new_v4();
        let shard_id = router.get_shard_id(&vector_id, None);

        assert!(shard_id < 4);

        let shard_nodes = router.get_shard_nodes(shard_id).unwrap();
        assert_eq!(shard_nodes.len(), 2); // replication_factor
    }

    #[test]
    fn test_consistent_hashing() {
        let nodes = vec!["node1".to_string(), "node2".to_string(), "node3".to_string()];
        let ring = ConsistentHashRing::new(nodes, 100);

        let node1 = ring.get_node("key1").unwrap();
        let node2 = ring.get_node("key2").unwrap();

        // Same key should always map to same node
        assert_eq!(ring.get_node("key1").unwrap(), node1);

        // Get multiple nodes for replication
        let replica_nodes = ring.get_nodes("key1", 3);
        assert_eq!(replica_nodes.len(), 3);
    }

    #[test]
    fn test_custom_shard_key() {
        let config = ShardingConfig {
            shard_count: 4,
            method: ShardingMethod::Custom,
            replication_factor: 1,
        };

        let nodes = vec!["node1".to_string()];
        let router = ShardRouter::new(config, nodes);

        let vector_id = Uuid::new_v4();

        // Same shard key should produce same shard
        let shard1 = router.get_shard_id(&vector_id, Some("user:123"));
        let shard2 = router.get_shard_id(&vector_id, Some("user:123"));
        assert_eq!(shard1, shard2);

        // Different shard key might produce different shard
        let shard3 = router.get_shard_id(&vector_id, Some("user:456"));
        // Note: might be same by chance, but logic is correct
        assert!(shard3 < 4);
    }
}
