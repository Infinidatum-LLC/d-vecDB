use crate::{VectorDbClient, ClientConfig, ServerStats};
use vectordb_common::{Result, VectorDbError};
use vectordb_common::types::{Vector, VectorId, CollectionId, IndexConfig};
use vectordb_common::types::{CollectionConfig as CommonCollectionConfig, CollectionStats as CommonCollectionStats};
use vectordb_common::types::{QueryRequest, QueryResult};
use vectordb_proto::{vector_db_client::VectorDbClient as ProtoClient};
use vectordb_proto::{
    CreateCollectionRequest, DeleteCollectionRequest, GetCollectionInfoRequest,
    ListCollectionsRequest, InsertRequest, DeleteRequest, BatchInsertRequest,
    UpdateRequest, GetStatsRequest, HealthRequest,
    Vector as ProtoVector
};
use tonic::transport::{Channel, Endpoint};
use tonic::{Request, Status};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{info, warn, instrument};
use uuid::Uuid;

/// gRPC client implementation
pub struct GrpcClient {
    client: ProtoClient<Channel>,
    config: ClientConfig,
}

impl GrpcClient {
    /// Create a new gRPC client
    pub async fn new(config: ClientConfig) -> Result<Self> {
        config.validate().map_err(|e| VectorDbError::ConfigError {
            message: e.to_string(),
        })?;
        
        let endpoint = Endpoint::from_shared(config.endpoint.clone())
            .map_err(|e| VectorDbError::NetworkError {
                message: format!("Invalid endpoint: {}", e),
            })?
            .timeout(Duration::from_secs(config.timeout_seconds))
            .user_agent(&config.user_agent)
            .map_err(|e| VectorDbError::NetworkError {
                message: format!("Failed to set user agent: {}", e),
            })?;

        let channel = endpoint.connect().await.map_err(|e| VectorDbError::NetworkError {
            message: format!("Failed to connect: {}", e),
        })?;

        let client = ProtoClient::new(channel);

        info!("Connected to VectorDB gRPC server at {}", config.endpoint);
        
        Ok(Self { client, config })
    }

    /// Execute with retry logic
    async fn with_retry<T, F, Fut>(&self, operation: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = std::result::Result<T, Status>>,
    {
        let mut last_error = None;
        
        for attempt in 0..=self.config.max_retries {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(status) => {
                    last_error = Some(status.clone());
                    
                    if attempt < self.config.max_retries {
                        let should_retry = match status.code() {
                            tonic::Code::Unavailable
                            | tonic::Code::DeadlineExceeded
                            | tonic::Code::ResourceExhausted => true,
                            _ => false,
                        };
                        
                        if should_retry {
                            warn!("gRPC request failed, retrying (attempt {}/{}): {}", 
                                  attempt + 1, self.config.max_retries, status);
                            tokio::time::sleep(Duration::from_millis(self.config.retry_delay_ms)).await;
                            continue;
                        }
                    }
                    
                    break;
                }
            }
        }
        
        Err(VectorDbError::NetworkError {
            message: format!("Request failed after {} retries: {}", 
                           self.config.max_retries, 
                           last_error.unwrap()),
        })
    }
}

#[async_trait::async_trait]
impl VectorDbClient for GrpcClient {
    #[instrument(skip(self))]
    async fn create_collection(&self, config: &CommonCollectionConfig) -> Result<()> {
        let proto_config = vectordb_proto::CollectionConfig {
            name: config.name.clone(),
            dimension: config.dimension as u32,
            distance_metric: config.distance_metric.into(),
            vector_type: config.vector_type.into(),
            index_config: Some(vectordb_proto::IndexConfig {
                max_connections: config.index_config.max_connections as u32,
                ef_construction: config.index_config.ef_construction as u32,
                ef_search: config.index_config.ef_search as u32,
                max_layer: config.index_config.max_layer as u32,
            }),
        };

        let request = CreateCollectionRequest {
            config: Some(proto_config),
        };

        let response = self.with_retry(|| async {
            let mut client = self.client.clone();
            client.create_collection(Request::new(request.clone())).await
        }).await?;

        let response = response.into_inner();
        if !response.success {
            return Err(VectorDbError::Internal {
                message: response.message,
            });
        }

        Ok(())
    }

    #[instrument(skip(self))]
    async fn delete_collection(&self, name: &str) -> Result<()> {
        let request = DeleteCollectionRequest {
            collection_name: name.to_string(),
        };

        let response = self.with_retry(|| async {
            let mut client = self.client.clone();
            client.delete_collection(Request::new(request.clone())).await
        }).await?;

        let response = response.into_inner();
        if !response.success {
            return Err(VectorDbError::Internal {
                message: response.message,
            });
        }

        Ok(())
    }

    #[instrument(skip(self))]
    async fn list_collections(&self) -> Result<Vec<CollectionId>> {
        let request = ListCollectionsRequest {};

        let response = self.with_retry(|| async {
            let mut client = self.client.clone();
            client.list_collections(Request::new(request.clone())).await
        }).await?;

        Ok(response.into_inner().collection_names)
    }

    #[instrument(skip(self))]
    async fn get_collection_info(&self, name: &str) -> Result<(CommonCollectionConfig, CommonCollectionStats)> {
        let request = GetCollectionInfoRequest {
            collection_name: name.to_string(),
        };

        let response = self.with_retry(|| async {
            let mut client = self.client.clone();
            client.get_collection_info(Request::new(request.clone())).await
        }).await?;

        let response = response.into_inner();
        
        let proto_config = response.config.ok_or_else(|| VectorDbError::Internal {
            message: "Missing collection config in response".to_string(),
        })?;

        let proto_stats = response.stats.ok_or_else(|| VectorDbError::Internal {
            message: "Missing collection stats in response".to_string(),
        })?;

        let config = CommonCollectionConfig {
            name: proto_config.name.clone(),
            dimension: proto_config.dimension as usize,
            distance_metric: proto_config.distance_metric().into(),
            vector_type: proto_config.vector_type().into(),
            index_config: proto_config.index_config.map_or(IndexConfig::default(), |ic| {
                IndexConfig {
                    max_connections: ic.max_connections as usize,
                    ef_construction: ic.ef_construction as usize,
                    ef_search: ic.ef_search as usize,
                    max_layer: ic.max_layer as usize,
                }
            }),
        };

        let stats = CommonCollectionStats {
            name: proto_stats.name,
            vector_count: proto_stats.vector_count as usize,
            dimension: proto_stats.dimension as usize,
            index_size: proto_stats.index_size as usize,
            memory_usage: proto_stats.memory_usage as usize,
        };

        Ok((config, stats))
    }

    #[instrument(skip(self, vector))]
    async fn insert(&self, collection: &str, vector: &Vector) -> Result<()> {
        let proto_vector = vectordb_proto::Vector {
            id: vector.id.to_string(),
            data: vector.data.clone(),
            metadata: vector.metadata.as_ref().map_or(HashMap::new(), |meta| {
                meta.iter()
                    .map(|(k, v)| (k.clone(), v.to_string()))
                    .collect()
            }),
        };

        let request = InsertRequest {
            collection_name: collection.to_string(),
            vector: Some(proto_vector),
        };

        let response = self.with_retry(|| async {
            let mut client = self.client.clone();
            client.insert(Request::new(request.clone())).await
        }).await?;

        let response = response.into_inner();
        if !response.success {
            return Err(VectorDbError::Internal {
                message: response.message,
            });
        }

        Ok(())
    }

    #[instrument(skip(self, vectors))]
    async fn batch_insert(&self, collection: &str, vectors: &[Vector]) -> Result<()> {
        let proto_vectors: Vec<ProtoVector> = vectors
            .iter()
            .map(|v| ProtoVector {
                id: v.id.to_string(),
                data: v.data.clone(),
                metadata: v.metadata.as_ref().map_or(HashMap::new(), |meta| {
                    meta.iter()
                        .map(|(k, v)| (k.clone(), v.to_string()))
                        .collect()
                }),
            })
            .collect();

        let request = BatchInsertRequest {
            collection_name: collection.to_string(),
            vectors: proto_vectors,
        };

        let response = self.with_retry(|| async {
            let mut client = self.client.clone();
            client.batch_insert(Request::new(request.clone())).await
        }).await?;

        let response = response.into_inner();
        if !response.success {
            return Err(VectorDbError::Internal {
                message: response.message,
            });
        }

        Ok(())
    }

    #[instrument(skip(self, request))]
    async fn query(&self, request: &QueryRequest) -> Result<Vec<QueryResult>> {
        let proto_request = vectordb_proto::QueryRequest {
            collection_name: request.collection.clone(),
            query_vector: request.vector.clone(),
            limit: request.limit as u32,
            ef_search: request.ef_search.map(|ef| ef as u32),
            filter: request.filter.as_ref().map_or(HashMap::new(), |filter| {
                filter.iter()
                    .map(|(k, v)| (k.clone(), v.to_string()))
                    .collect()
            }),
        };

        let response = self.with_retry(|| async {
            let mut client = self.client.clone();
            client.query(Request::new(proto_request.clone())).await
        }).await?;

        let response = response.into_inner();
        
        let results = response
            .results
            .into_iter()
            .map(|r| {
                let id = Uuid::parse_str(&r.id).map_err(|_| VectorDbError::Internal {
                    message: format!("Invalid UUID in response: {}", r.id),
                })?;

                let metadata = if r.metadata.is_empty() {
                    None
                } else {
                    Some(
                        r.metadata
                            .into_iter()
                            .map(|(k, v)| (k, serde_json::Value::String(v)))
                            .collect(),
                    )
                };

                Ok(QueryResult {
                    id,
                    distance: r.distance,
                    metadata,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(results)
    }

    #[instrument(skip(self))]
    async fn get(&self, _collection: &str, _id: &VectorId) -> Result<Option<Vector>> {
        // gRPC service doesn't currently support get by ID
        // This would need to be added to the proto definition
        Err(VectorDbError::Internal {
            message: "Get by ID not supported in gRPC client".to_string(),
        })
    }

    #[instrument(skip(self, vector))]
    async fn update(&self, collection: &str, vector: &Vector) -> Result<()> {
        let proto_vector = vectordb_proto::Vector {
            id: vector.id.to_string(),
            data: vector.data.clone(),
            metadata: vector.metadata.as_ref().map_or(HashMap::new(), |meta| {
                meta.iter()
                    .map(|(k, v)| (k.clone(), v.to_string()))
                    .collect()
            }),
        };

        let request = UpdateRequest {
            collection_name: collection.to_string(),
            vector: Some(proto_vector),
        };

        let response = self.with_retry(|| async {
            let mut client = self.client.clone();
            client.update(Request::new(request.clone())).await
        }).await?;

        let response = response.into_inner();
        if !response.success {
            return Err(VectorDbError::Internal {
                message: response.message,
            });
        }

        Ok(())
    }

    #[instrument(skip(self))]
    async fn delete(&self, collection: &str, id: &VectorId) -> Result<bool> {
        let request = DeleteRequest {
            collection_name: collection.to_string(),
            vector_id: id.to_string(),
        };

        let response = self.with_retry(|| async {
            let mut client = self.client.clone();
            client.delete(Request::new(request.clone())).await
        }).await?;

        let response = response.into_inner();
        Ok(response.success)
    }

    #[instrument(skip(self))]
    async fn get_stats(&self) -> Result<ServerStats> {
        let request = GetStatsRequest {};

        let response = self.with_retry(|| async {
            let mut client = self.client.clone();
            client.get_stats(Request::new(request.clone())).await
        }).await?;

        let response = response.into_inner();
        let stats = response.stats.ok_or_else(|| VectorDbError::Internal {
            message: "Missing server stats in response".to_string(),
        })?;

        Ok(ServerStats {
            total_vectors: stats.total_vectors,
            total_collections: stats.total_collections,
            memory_usage: stats.memory_usage,
            disk_usage: stats.disk_usage,
            uptime_seconds: stats.uptime_seconds,
        })
    }

    #[instrument(skip(self))]
    async fn health(&self) -> Result<bool> {
        let request = HealthRequest {};

        match self.with_retry(|| async {
            let mut client = self.client.clone();
            client.health(Request::new(request.clone())).await
        }).await {
            Ok(response) => Ok(response.into_inner().healthy),
            Err(_) => Ok(false),
        }
    }

    // Advanced Search APIs

    #[instrument(skip(self))]
    async fn recommend(&self, request: &vectordb_common::search_api::RecommendRequest) -> Result<Vec<QueryResult>> {
        let proto_request = vectordb_proto::RecommendRequest {
            collection_name: request.collection.clone(),
            positive_ids: request.positive.iter().map(|id| id.to_string()).collect(),
            negative_ids: request.negative.iter().map(|id| id.to_string()).collect(),
            filter_json: request.filter.as_ref().map(|f| serde_json::to_string(f).unwrap_or_default()),
            limit: request.limit as u32,
            offset: request.offset as u32,
            strategy: match request.strategy {
                vectordb_common::search_api::RecommendStrategy::AverageVector => "average_vector".to_string(),
                vectordb_common::search_api::RecommendStrategy::BestScore => "best_score".to_string(),
            },
        };

        let response = self.with_retry(|| async {
            let mut client = self.client.clone();
            client.recommend(Request::new(proto_request.clone())).await
        }).await?;

        let response = response.into_inner();
        let results = response
            .results
            .into_iter()
            .map(|r| {
                let id = Uuid::parse_str(&r.id).map_err(|_| VectorDbError::Internal {
                    message: format!("Invalid UUID in response: {}", r.id),
                })?;

                let metadata = if r.metadata.is_empty() {
                    None
                } else {
                    Some(
                        r.metadata
                            .into_iter()
                            .map(|(k, v)| (k, serde_json::Value::String(v)))
                            .collect(),
                    )
                };

                Ok(QueryResult {
                    id,
                    distance: r.distance,
                    metadata,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(results)
    }

    #[instrument(skip(self))]
    async fn discover(&self, request: &vectordb_common::search_api::DiscoveryRequest) -> Result<Vec<QueryResult>> {
        use vectordb_proto::discover_request::Target;

        let target = match &request.target {
            vectordb_common::search_api::DiscoveryTarget::VectorId(id) => {
                Some(Target::TargetId(id.to_string()))
            }
            vectordb_common::search_api::DiscoveryTarget::Vector(vec) => {
                Some(Target::TargetVector(vectordb_proto::TargetVector {
                    data: vec.clone()
                }))
            }
        };

        let proto_request = vectordb_proto::DiscoverRequest {
            collection_name: request.collection.clone(),
            target,
            context_pairs: request
                .context
                .iter()
                .map(|pair| vectordb_proto::ContextPair {
                    positive_id: pair.positive.to_string(),
                    negative_id: pair.negative.to_string(),
                })
                .collect(),
            filter_json: request.filter.as_ref().map(|f| serde_json::to_string(f).unwrap_or_default()),
            limit: request.limit as u32,
            offset: request.offset as u32,
        };

        let response = self.with_retry(|| async {
            let mut client = self.client.clone();
            client.discover(Request::new(proto_request.clone())).await
        }).await?;

        let response = response.into_inner();
        let results = response
            .results
            .into_iter()
            .map(|r| {
                let id = Uuid::parse_str(&r.id).map_err(|_| VectorDbError::Internal {
                    message: format!("Invalid UUID in response: {}", r.id),
                })?;

                let metadata = if r.metadata.is_empty() {
                    None
                } else {
                    Some(
                        r.metadata
                            .into_iter()
                            .map(|(k, v)| (k, serde_json::Value::String(v)))
                            .collect(),
                    )
                };

                Ok(QueryResult {
                    id,
                    distance: r.distance,
                    metadata,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(results)
    }

    #[instrument(skip(self))]
    async fn scroll(&self, request: &vectordb_common::search_api::ScrollRequest) -> Result<vectordb_common::search_api::ScrollResponse> {
        let proto_request = vectordb_proto::ScrollRequest {
            collection_name: request.collection.clone(),
            filter_json: request.filter.as_ref().map(|f| serde_json::to_string(f).unwrap_or_default()),
            limit: request.limit as u32,
            offset: request.offset.clone(),
            with_vectors: request.with_vectors,
            with_payload: request.with_payload,
        };

        let response = self.with_retry(|| async {
            let mut client = self.client.clone();
            client.scroll(Request::new(proto_request.clone())).await
        }).await?;

        let response = response.into_inner();
        let points = response
            .points
            .into_iter()
            .map(|p| {
                let id = Uuid::parse_str(&p.id).map_err(|_| VectorDbError::Internal {
                    message: format!("Invalid UUID in response: {}", p.id),
                })?;

                let payload = if p.payload.is_empty() {
                    None
                } else {
                    Some(
                        p.payload
                            .into_iter()
                            .map(|(k, v)| (k, serde_json::Value::String(v)))
                            .collect(),
                    )
                };

                Ok(vectordb_common::search_api::ScoredPoint {
                    id,
                    score: p.score,
                    vector: if p.vector.is_empty() { None } else { Some(p.vector) },
                    payload,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(vectordb_common::search_api::ScrollResponse {
            points,
            next_offset: response.next_offset,
        })
    }

    #[instrument(skip(self))]
    async fn count(&self, request: &vectordb_common::search_api::CountRequest) -> Result<vectordb_common::search_api::CountResponse> {
        let proto_request = vectordb_proto::CountRequest {
            collection_name: request.collection.clone(),
            filter_json: request.filter.as_ref().map(|f| serde_json::to_string(f).unwrap_or_default()),
            exact: request.exact,
        };

        let response = self.with_retry(|| async {
            let mut client = self.client.clone();
            client.count(Request::new(proto_request.clone())).await
        }).await?;

        let response = response.into_inner();
        Ok(vectordb_common::search_api::CountResponse {
            count: response.count as usize,
        })
    }

    #[instrument(skip(self))]
    async fn batch_search(&self, request: &vectordb_common::search_api::BatchSearchRequest) -> Result<Vec<Vec<QueryResult>>> {
        let proto_request = vectordb_proto::BatchSearchRequest {
            collection_name: request.collection.clone(),
            searches: request
                .searches
                .iter()
                .map(|s| vectordb_proto::SearchQuery {
                    vector: s.vector.clone(),
                    filter_json: s.filter.as_ref().map(|f| serde_json::to_string(f).unwrap_or_default()),
                    limit: s.limit as u32,
                    offset: s.offset as u32,
                })
                .collect(),
        };

        let response = self.with_retry(|| async {
            let mut client = self.client.clone();
            client.batch_search(Request::new(proto_request.clone())).await
        }).await?;

        let response = response.into_inner();
        let batch_results = response
            .results
            .into_iter()
            .map(|query_results| {
                query_results
                    .results
                    .into_iter()
                    .map(|r| {
                        let id = Uuid::parse_str(&r.id).map_err(|_| VectorDbError::Internal {
                            message: format!("Invalid UUID in response: {}", r.id),
                        })?;

                        let metadata = if r.metadata.is_empty() {
                            None
                        } else {
                            Some(
                                r.metadata
                                    .into_iter()
                                    .map(|(k, v)| (k, serde_json::Value::String(v)))
                                    .collect(),
                            )
                        };

                        Ok(QueryResult {
                            id,
                            distance: r.distance,
                            metadata,
                        })
                    })
                    .collect::<Result<Vec<_>>>()
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(batch_results)
    }

    // Snapshot Management APIs

    #[instrument(skip(self))]
    async fn create_snapshot(&self, collection: &str) -> Result<vectordb_storage::SnapshotMetadata> {
        let proto_request = vectordb_proto::CreateSnapshotRequest {
            collection_name: collection.to_string(),
        };

        let response = self.with_retry(|| async {
            let mut client = self.client.clone();
            client.create_snapshot(Request::new(proto_request.clone())).await
        }).await?;

        let response = response.into_inner();
        let metadata = response.metadata.ok_or_else(|| VectorDbError::Internal {
            message: "Missing snapshot metadata in response".to_string(),
        })?;

        Ok(vectordb_storage::SnapshotMetadata {
            name: metadata.name,
            collection: metadata.collection,
            created_at: metadata.created_at,
            size_bytes: metadata.size_bytes,
            vector_count: metadata.vector_count as usize,
            checksum: metadata.checksum,
        })
    }

    #[instrument(skip(self))]
    async fn list_snapshots(&self, collection: &str) -> Result<Vec<vectordb_storage::SnapshotMetadata>> {
        let proto_request = vectordb_proto::ListSnapshotsRequest {
            collection_name: collection.to_string(),
        };

        let response = self.with_retry(|| async {
            let mut client = self.client.clone();
            client.list_snapshots(Request::new(proto_request.clone())).await
        }).await?;

        let response = response.into_inner();
        let snapshots = response
            .snapshots
            .into_iter()
            .map(|metadata| vectordb_storage::SnapshotMetadata {
                name: metadata.name,
                collection: metadata.collection,
                created_at: metadata.created_at,
                size_bytes: metadata.size_bytes,
                vector_count: metadata.vector_count as usize,
                checksum: metadata.checksum,
            })
            .collect();

        Ok(snapshots)
    }

    #[instrument(skip(self))]
    async fn get_snapshot(&self, collection: &str, snapshot_name: &str) -> Result<vectordb_storage::SnapshotMetadata> {
        let proto_request = vectordb_proto::GetSnapshotRequest {
            collection_name: collection.to_string(),
            snapshot_name: snapshot_name.to_string(),
        };

        let response = self.with_retry(|| async {
            let mut client = self.client.clone();
            client.get_snapshot(Request::new(proto_request.clone())).await
        }).await?;

        let response = response.into_inner();
        let metadata = response.metadata.ok_or_else(|| VectorDbError::Internal {
            message: "Missing snapshot metadata in response".to_string(),
        })?;

        Ok(vectordb_storage::SnapshotMetadata {
            name: metadata.name,
            collection: metadata.collection,
            created_at: metadata.created_at,
            size_bytes: metadata.size_bytes,
            vector_count: metadata.vector_count as usize,
            checksum: metadata.checksum,
        })
    }

    #[instrument(skip(self))]
    async fn delete_snapshot(&self, collection: &str, snapshot_name: &str) -> Result<()> {
        let proto_request = vectordb_proto::DeleteSnapshotRequest {
            collection_name: collection.to_string(),
            snapshot_name: snapshot_name.to_string(),
        };

        let response = self.with_retry(|| async {
            let mut client = self.client.clone();
            client.delete_snapshot(Request::new(proto_request.clone())).await
        }).await?;

        let response = response.into_inner();
        if !response.success {
            return Err(VectorDbError::Internal {
                message: response.message,
            });
        }

        Ok(())
    }

    #[instrument(skip(self))]
    async fn restore_snapshot(&self, collection: &str, snapshot_name: &str) -> Result<()> {
        let proto_request = vectordb_proto::RestoreSnapshotRequest {
            collection_name: collection.to_string(),
            snapshot_name: snapshot_name.to_string(),
        };

        let response = self.with_retry(|| async {
            let mut client = self.client.clone();
            client.restore_snapshot(Request::new(proto_request.clone())).await
        }).await?;

        let response = response.into_inner();
        if !response.success {
            return Err(VectorDbError::Internal {
                message: response.message,
            });
        }

        Ok(())
    }
}