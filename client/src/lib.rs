pub mod grpc_client;
pub mod rest_client;
pub mod config;

use vectordb_common::Result;
use vectordb_common::types::*;

pub use config::*;
pub use grpc_client::*;
pub use rest_client::*;

/// Client interface for VectorDB
#[async_trait::async_trait]
pub trait VectorDbClient: Send + Sync {
    /// Create a new collection
    async fn create_collection(&self, config: &CollectionConfig) -> Result<()>;

    /// Delete a collection
    async fn delete_collection(&self, name: &str) -> Result<()>;

    /// List all collections
    async fn list_collections(&self) -> Result<Vec<CollectionId>>;

    /// Get collection information
    async fn get_collection_info(&self, name: &str) -> Result<(CollectionConfig, CollectionStats)>;

    /// Insert a single vector
    async fn insert(&self, collection: &str, vector: &Vector) -> Result<()>;

    /// Insert multiple vectors
    async fn batch_insert(&self, collection: &str, vectors: &[Vector]) -> Result<()>;

    /// Query for nearest neighbors
    async fn query(&self, request: &QueryRequest) -> Result<Vec<QueryResult>>;

    /// Get a vector by ID
    async fn get(&self, collection: &str, id: &VectorId) -> Result<Option<Vector>>;

    /// Update a vector
    async fn update(&self, collection: &str, vector: &Vector) -> Result<()>;

    /// Delete a vector
    async fn delete(&self, collection: &str, id: &VectorId) -> Result<bool>;

    /// Get server statistics
    async fn get_stats(&self) -> Result<ServerStats>;

    /// Check server health
    async fn health(&self) -> Result<bool>;

    // Advanced Search APIs

    /// Recommend vectors based on positive and negative examples
    async fn recommend(&self, request: &vectordb_common::search_api::RecommendRequest) -> Result<Vec<QueryResult>>;

    /// Discovery search - find vectors between positive and negative context
    async fn discover(&self, request: &vectordb_common::search_api::DiscoveryRequest) -> Result<Vec<QueryResult>>;

    /// Scroll through all vectors with optional filtering
    async fn scroll(&self, request: &vectordb_common::search_api::ScrollRequest) -> Result<vectordb_common::search_api::ScrollResponse>;

    /// Count vectors matching filter criteria
    async fn count(&self, request: &vectordb_common::search_api::CountRequest) -> Result<vectordb_common::search_api::CountResponse>;

    /// Batch search - execute multiple queries in one request
    async fn batch_search(&self, request: &vectordb_common::search_api::BatchSearchRequest) -> Result<Vec<Vec<QueryResult>>>;

    // Snapshot Management APIs

    /// Create a snapshot of a collection
    async fn create_snapshot(&self, collection: &str) -> Result<vectordb_storage::SnapshotMetadata>;

    /// List all snapshots for a collection
    async fn list_snapshots(&self, collection: &str) -> Result<Vec<vectordb_storage::SnapshotMetadata>>;

    /// Get snapshot metadata
    async fn get_snapshot(&self, collection: &str, snapshot_name: &str) -> Result<vectordb_storage::SnapshotMetadata>;

    /// Delete a snapshot
    async fn delete_snapshot(&self, collection: &str, snapshot_name: &str) -> Result<()>;

    /// Restore collection from snapshot
    async fn restore_snapshot(&self, collection: &str, snapshot_name: &str) -> Result<()>;
}

/// Server statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ServerStats {
    pub total_vectors: u64,
    pub total_collections: u32,
    pub memory_usage: u64,
    pub disk_usage: u64,
    pub uptime_seconds: u64,
}

/// Create a client based on configuration
pub async fn create_client(config: ClientConfig) -> Result<Box<dyn VectorDbClient>> {
    match config.protocol {
        ClientProtocol::Grpc => {
            let client = GrpcClient::new(config).await?;
            Ok(Box::new(client))
        }
        ClientProtocol::Rest => {
            let client = RestClient::new(config)?;
            Ok(Box::new(client))
        }
    }
}

/// Convenience client builder
pub struct ClientBuilder {
    config: ClientConfig,
}

impl ClientBuilder {
    pub fn new() -> Self {
        Self {
            config: ClientConfig::default(),
        }
    }
    
    pub fn grpc<S: Into<String>>(mut self, endpoint: S) -> Self {
        self.config.protocol = ClientProtocol::Grpc;
        self.config.endpoint = endpoint.into();
        self
    }
    
    pub fn rest<S: Into<String>>(mut self, endpoint: S) -> Self {
        self.config.protocol = ClientProtocol::Rest;
        self.config.endpoint = endpoint.into();
        self
    }
    
    pub fn timeout(mut self, timeout_seconds: u64) -> Self {
        self.config.timeout_seconds = timeout_seconds;
        self
    }
    
    pub fn max_retries(mut self, max_retries: u32) -> Self {
        self.config.max_retries = max_retries;
        self
    }
    
    pub async fn build(self) -> Result<Box<dyn VectorDbClient>> {
        create_client(self.config).await
    }
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_client_builder() {
        let builder = ClientBuilder::new()
            .grpc("http://localhost:9090")
            .timeout(30)
            .max_retries(3);
        
        assert_eq!(builder.config.protocol, ClientProtocol::Grpc);
        assert_eq!(builder.config.endpoint, "http://localhost:9090");
        assert_eq!(builder.config.timeout_seconds, 30);
        assert_eq!(builder.config.max_retries, 3);
    }
}