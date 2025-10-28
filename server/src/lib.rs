pub mod grpc;
pub mod rest;
pub mod config;
pub mod metrics;
pub mod health;

use vectordb_vectorstore::VectorStore;
use std::sync::Arc;
use anyhow::Result;
use tracing::{info, error};

pub use config::*;
pub use grpc::*;
pub use rest::*;
pub use metrics::*;
pub use health::*;

/// Main server application
pub struct VectorDbServer {
    config: ServerConfig,
    store: Arc<VectorStore>,
}

impl VectorDbServer {
    /// Create a new server instance
    pub async fn new(config: ServerConfig) -> Result<Self> {
        info!("Initializing VectorDB server");
        
        // Create vector store
        let store = Arc::new(VectorStore::new(&config.data_dir).await?);
        
        info!("VectorDB server initialized successfully");
        
        Ok(Self { config, store })
    }
    
    /// Start the server (both gRPC and REST)
    pub async fn start(self) -> Result<()> {
        info!("Starting VectorDB server on {}:{}", self.config.host, self.config.grpc_port);
        
        // Initialize metrics exporter
        let metrics_handle = self.start_metrics_server().await?;
        
        // Start gRPC server
        let grpc_handle = {
            let store = Arc::clone(&self.store);
            let grpc_addr = format!("{}:{}", self.config.host, self.config.grpc_port)
                .parse()
                .expect("Invalid gRPC address");
            
            tokio::spawn(async move {
                if let Err(e) = start_grpc_server(grpc_addr, store).await {
                    error!("gRPC server error: {}", e);
                }
            })
        };
        
        // Start REST server
        let rest_handle = {
            let store = Arc::clone(&self.store);
            let rest_addr = format!("{}:{}", self.config.host, self.config.rest_port)
                .parse()
                .expect("Invalid REST address");
            
            tokio::spawn(async move {
                if let Err(e) = start_rest_server(rest_addr, store).await {
                    error!("REST server error: {}", e);
                }
            })
        };
        
        info!("VectorDB servers started");
        info!("gRPC API: {}:{}", self.config.host, self.config.grpc_port);
        info!("REST API: {}:{}", self.config.host, self.config.rest_port);
        info!("Metrics: {}:{}", self.config.host, self.config.metrics_port);
        
        // Wait for either server to complete
        tokio::select! {
            result = grpc_handle => {
                if let Err(e) = result {
                    error!("gRPC server task failed: {}", e);
                }
            }
            result = rest_handle => {
                if let Err(e) = result {
                    error!("REST server task failed: {}", e);
                }
            }
            result = metrics_handle => {
                if let Err(e) = result {
                    error!("Metrics server task failed: {}", e);
                }
            }
        }
        
        Ok(())
    }
    
    /// Start metrics server
    async fn start_metrics_server(&self) -> Result<tokio::task::JoinHandle<()>> {
        let metrics_addr = format!("{}:{}", self.config.host, self.config.metrics_port)
            .parse()
            .expect("Invalid metrics address");
        
        let handle = tokio::spawn(async move {
            if let Err(e) = start_metrics_server(metrics_addr).await {
                error!("Metrics server error: {}", e);
            }
        });
        
        Ok(handle)
    }
}