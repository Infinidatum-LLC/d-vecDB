use crate::{VectorDbClient, ClientConfig, ServerStats};
use vectordb_common::{Result, VectorDbError};
use vectordb_common::types::*;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{info, warn, instrument};

/// REST API response wrapper
#[derive(Deserialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

/// REST client implementation
pub struct RestClient {
    client: Client,
    base_url: String,
    config: ClientConfig,
}

impl RestClient {
    /// Create a new REST client
    pub fn new(config: ClientConfig) -> Result<Self> {
        config.validate().map_err(|e| VectorDbError::ConfigError {
            message: e.to_string(),
        })?;

        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .user_agent(&config.user_agent)
            .gzip(config.enable_compression)
            .build()
            .map_err(|e| VectorDbError::NetworkError {
                message: format!("Failed to create HTTP client: {}", e),
            })?;

        let base_url = config.endpoint.trim_end_matches('/').to_string();

        info!("Connected to VectorDB REST API at {}", base_url);

        Ok(Self {
            client,
            base_url,
            config,
        })
    }

    /// Execute HTTP request with retry logic
    async fn request_with_retry<T: for<'de> Deserialize<'de>>(
        &self,
        request_builder: reqwest::RequestBuilder,
    ) -> Result<T> {
        let mut last_error = None;

        for attempt in 0..=self.config.max_retries {
            let request = request_builder
                .try_clone()
                .ok_or_else(|| VectorDbError::Internal {
                    message: "Failed to clone request".to_string(),
                })?;

            match request.send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        match response.json::<ApiResponse<T>>().await {
                            Ok(api_response) => {
                                if api_response.success {
                                    if let Some(data) = api_response.data {
                                        return Ok(data);
                                    } else {
                                        return Err(VectorDbError::Internal {
                                            message: "Missing data in successful response".to_string(),
                                        });
                                    }
                                } else {
                                    return Err(VectorDbError::Internal {
                                        message: api_response.error.unwrap_or_else(|| "Unknown error".to_string()),
                                    });
                                }
                            }
                            Err(e) => {
                                return Err(VectorDbError::Serialization(e.to_string()));
                            }
                        }
                    } else {
                        let status = response.status();
                        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                        
                        let error = VectorDbError::NetworkError {
                            message: format!("HTTP {}: {}", status, error_text),
                        };

                        if attempt < self.config.max_retries {
                            let should_retry = status.is_server_error() || status == reqwest::StatusCode::TOO_MANY_REQUESTS;
                            
                            if should_retry {
                                warn!("HTTP request failed, retrying (attempt {}/{}): {}", 
                                      attempt + 1, self.config.max_retries, error);
                                tokio::time::sleep(Duration::from_millis(self.config.retry_delay_ms)).await;
                                last_error = Some(error);
                                continue;
                            }
                        }

                        return Err(error);
                    }
                }
                Err(e) => {
                    last_error = Some(VectorDbError::NetworkError {
                        message: e.to_string(),
                    });

                    if attempt < self.config.max_retries {
                        warn!("HTTP request failed, retrying (attempt {}/{}): {}", 
                              attempt + 1, self.config.max_retries, e);
                        tokio::time::sleep(Duration::from_millis(self.config.retry_delay_ms)).await;
                        continue;
                    }

                    break;
                }
            }
        }

        Err(last_error.unwrap_or_else(|| VectorDbError::NetworkError {
            message: "Unknown network error".to_string(),
        }))
    }
}

#[async_trait::async_trait]
impl VectorDbClient for RestClient {
    #[instrument(skip(self))]
    async fn create_collection(&self, config: &CollectionConfig) -> Result<()> {
        #[derive(Serialize)]
        struct CreateCollectionRequest {
            name: String,
            dimension: usize,
            distance_metric: DistanceMetric,
            vector_type: VectorType,
            index_config: Option<IndexConfig>,
            quantization: Option<vectordb_common::quantization::QuantizationConfig>,
        }

        let request_body = CreateCollectionRequest {
            name: config.name.clone(),
            dimension: config.dimension,
            distance_metric: config.distance_metric,
            vector_type: config.vector_type,
            index_config: Some(config.index_config.clone()),
            quantization: config.quantization.clone(),
        };

        let request = self.client
            .post(&format!("{}/collections", self.base_url))
            .json(&request_body);

        self.request_with_retry::<()>(request).await
    }

    #[instrument(skip(self))]
    async fn delete_collection(&self, name: &str) -> Result<()> {
        let request = self.client
            .delete(&format!("{}/collections/{}", self.base_url, name));

        self.request_with_retry::<()>(request).await
    }

    #[instrument(skip(self))]
    async fn list_collections(&self) -> Result<Vec<CollectionId>> {
        let request = self.client
            .get(&format!("{}/collections", self.base_url));

        self.request_with_retry::<Vec<String>>(request).await
    }

    #[instrument(skip(self))]
    async fn get_collection_info(&self, name: &str) -> Result<(CollectionConfig, CollectionStats)> {
        let request = self.client
            .get(&format!("{}/collections/{}", self.base_url, name));

        self.request_with_retry::<(CollectionConfig, CollectionStats)>(request).await
    }

    #[instrument(skip(self, vector))]
    async fn insert(&self, collection: &str, vector: &Vector) -> Result<()> {
        #[derive(Serialize)]
        struct InsertVectorRequest {
            id: Option<String>,
            data: Vec<f32>,
            metadata: Option<HashMap<String, serde_json::Value>>,
        }

        let request_body = InsertVectorRequest {
            id: Some(vector.id.to_string()),
            data: vector.data.clone(),
            metadata: vector.metadata.clone(),
        };

        let request = self.client
            .post(&format!("{}/collections/{}/vectors", self.base_url, collection))
            .json(&request_body);

        self.request_with_retry::<String>(request).await.map(|_| ())
    }

    #[instrument(skip(self, vectors))]
    async fn batch_insert(&self, collection: &str, vectors: &[Vector]) -> Result<()> {
        #[derive(Serialize)]
        struct InsertVectorRequest {
            id: Option<String>,
            data: Vec<f32>,
            metadata: Option<HashMap<String, serde_json::Value>>,
        }

        #[derive(Serialize)]
        struct BatchInsertRequest {
            vectors: Vec<InsertVectorRequest>,
        }

        let request_body = BatchInsertRequest {
            vectors: vectors
                .iter()
                .map(|v| InsertVectorRequest {
                    id: Some(v.id.to_string()),
                    data: v.data.clone(),
                    metadata: v.metadata.clone(),
                })
                .collect(),
        };

        let request = self.client
            .post(&format!("{}/collections/{}/vectors/batch", self.base_url, collection))
            .json(&request_body);

        self.request_with_retry::<Vec<String>>(request).await.map(|_| ())
    }

    #[instrument(skip(self, request))]
    async fn query(&self, request: &QueryRequest) -> Result<Vec<QueryResult>> {
        #[derive(Serialize)]
        struct QueryVectorsRequest {
            vector: Vec<f32>,
            limit: Option<usize>,
            ef_search: Option<usize>,
            filter: Option<HashMap<String, serde_json::Value>>,
        }

        let request_body = QueryVectorsRequest {
            vector: request.vector.clone(),
            limit: Some(request.limit),
            ef_search: request.ef_search,
            filter: request.filter.clone(),
        };

        let http_request = self.client
            .post(&format!("{}/collections/{}/search", self.base_url, request.collection))
            .json(&request_body);

        self.request_with_retry::<Vec<QueryResult>>(http_request).await
    }

    #[instrument(skip(self))]
    async fn get(&self, collection: &str, id: &VectorId) -> Result<Option<Vector>> {
        let request = self.client
            .get(&format!("{}/collections/{}/vectors/{}", self.base_url, collection, id));

        self.request_with_retry::<Option<Vector>>(request).await
    }

    #[instrument(skip(self, vector))]
    async fn update(&self, collection: &str, vector: &Vector) -> Result<()> {
        #[derive(Serialize)]
        struct InsertVectorRequest {
            id: Option<String>,
            data: Vec<f32>,
            metadata: Option<HashMap<String, serde_json::Value>>,
        }

        let request_body = InsertVectorRequest {
            id: None, // ID is in the URL path
            data: vector.data.clone(),
            metadata: vector.metadata.clone(),
        };

        let request = self.client
            .put(&format!("{}/collections/{}/vectors/{}", self.base_url, collection, vector.id))
            .json(&request_body);

        self.request_with_retry::<()>(request).await
    }

    #[instrument(skip(self))]
    async fn delete(&self, collection: &str, id: &VectorId) -> Result<bool> {
        let request = self.client
            .delete(&format!("{}/collections/{}/vectors/{}", self.base_url, collection, id));

        self.request_with_retry::<bool>(request).await
    }

    #[instrument(skip(self))]
    async fn get_stats(&self) -> Result<ServerStats> {
        let request = self.client
            .get(&format!("{}/stats", self.base_url));

        self.request_with_retry::<ServerStats>(request).await
    }

    #[instrument(skip(self))]
    async fn health(&self) -> Result<bool> {
        let request = self.client
            .get(&format!("{}/health", self.base_url));

        match self.request_with_retry::<String>(request).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}