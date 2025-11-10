use vectordb_proto::{
    vector_db_server::{VectorDb, VectorDbServer}, CreateCollectionRequest, CreateCollectionResponse,
    DeleteCollectionRequest, DeleteCollectionResponse, ListCollectionsRequest,
    ListCollectionsResponse, GetCollectionInfoRequest, GetCollectionInfoResponse,
    InsertRequest, InsertResponse, BatchInsertRequest, BatchInsertResponse,
    DeleteRequest, DeleteResponse, QueryRequest, QueryResponse, QueryResult,
    UpdateRequest, UpdateResponse, GetStatsRequest, GetStatsResponse,
    HealthRequest, HealthResponse
};
use vectordb_vectorstore::VectorStore;
use std::sync::Arc;
use std::collections::HashMap;
use tonic::{Request, Response, Status};
use tracing::{info, error, instrument};
use uuid::Uuid;
use std::net::SocketAddr;

/// gRPC service implementation
pub struct VectorDbService {
    store: Arc<VectorStore>,
}

impl VectorDbService {
    pub fn new(store: Arc<VectorStore>) -> Self {
        Self { store }
    }
}

#[tonic::async_trait]
impl VectorDb for VectorDbService {
    #[instrument(skip(self))]
    async fn create_collection(
        &self,
        request: Request<CreateCollectionRequest>,
    ) -> Result<Response<CreateCollectionResponse>, Status> {
        let req = request.into_inner();
        
        let config = req.config.ok_or_else(|| {
            Status::invalid_argument("Collection config is required")
        })?;
        
        let collection_config = vectordb_common::types::CollectionConfig {
            name: config.name.clone(),
            dimension: config.dimension as usize,
            distance_metric: config.distance_metric().into(),
            vector_type: config.vector_type().into(),
            index_config: config.index_config.map_or(vectordb_common::types::IndexConfig::default(), |ic| {
                vectordb_common::types::IndexConfig {
                    max_connections: ic.max_connections as usize,
                    ef_construction: ic.ef_construction as usize,
                    ef_search: ic.ef_search as usize,
                    max_layer: ic.max_layer as usize,
                }
            }),
            quantization: None,
        };
        
        match self.store.create_collection(&collection_config).await {
            Ok(()) => {
                info!("Created collection: {}", collection_config.name);
                Ok(Response::new(CreateCollectionResponse {
                    success: true,
                    message: "Collection created successfully".to_string(),
                }))
            }
            Err(e) => {
                error!("Failed to create collection: {}", e);
                Ok(Response::new(CreateCollectionResponse {
                    success: false,
                    message: e.to_string(),
                }))
            }
        }
    }
    
    #[instrument(skip(self))]
    async fn delete_collection(
        &self,
        request: Request<DeleteCollectionRequest>,
    ) -> Result<Response<DeleteCollectionResponse>, Status> {
        let req = request.into_inner();
        
        match self.store.delete_collection(&req.collection_name).await {
            Ok(()) => {
                info!("Deleted collection: {}", req.collection_name);
                Ok(Response::new(DeleteCollectionResponse {
                    success: true,
                    message: "Collection deleted successfully".to_string(),
                }))
            }
            Err(e) => {
                error!("Failed to delete collection: {}", e);
                Ok(Response::new(DeleteCollectionResponse {
                    success: false,
                    message: e.to_string(),
                }))
            }
        }
    }
    
    #[instrument(skip(self))]
    async fn list_collections(
        &self,
        _request: Request<ListCollectionsRequest>,
    ) -> Result<Response<ListCollectionsResponse>, Status> {
        let collections = self.store.list_collections();
        
        Ok(Response::new(ListCollectionsResponse {
            collection_names: collections,
        }))
    }
    
    #[instrument(skip(self))]
    async fn get_collection_info(
        &self,
        request: Request<GetCollectionInfoRequest>,
    ) -> Result<Response<GetCollectionInfoResponse>, Status> {
        let req = request.into_inner();
        
        let config = match self.store.get_collection_config(&req.collection_name) {
            Ok(Some(config)) => config,
            Ok(None) => return Err(Status::not_found("Collection not found")),
            Err(e) => return Err(Status::internal(e.to_string())),
        };
        
        let stats = match self.store.get_collection_stats(&req.collection_name).await {
            Ok(Some(stats)) => stats,
            Ok(None) => return Err(Status::not_found("Collection not found")),
            Err(e) => return Err(Status::internal(e.to_string())),
        };
        
        let proto_config = vectordb_proto::CollectionConfig {
            name: config.name,
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
        
        let proto_stats = vectordb_proto::CollectionStats {
            name: stats.name,
            vector_count: stats.vector_count as u64,
            dimension: stats.dimension as u32,
            index_size: stats.index_size as u64,
            memory_usage: stats.memory_usage as u64,
        };
        
        Ok(Response::new(GetCollectionInfoResponse {
            config: Some(proto_config),
            stats: Some(proto_stats),
        }))
    }
    
    #[instrument(skip(self))]
    async fn insert(
        &self,
        request: Request<InsertRequest>,
    ) -> Result<Response<InsertResponse>, Status> {
        let req = request.into_inner();
        
        let vector_proto = req.vector.ok_or_else(|| {
            Status::invalid_argument("Vector is required")
        })?;
        
        let vector_id = Uuid::parse_str(&vector_proto.id)
            .map_err(|_| Status::invalid_argument("Invalid vector ID format"))?;
        
        let metadata = if vector_proto.metadata.is_empty() {
            None
        } else {
            Some(
                vector_proto
                    .metadata
                    .into_iter()
                    .map(|(k, v)| (k, serde_json::Value::String(v)))
                    .collect::<HashMap<String, serde_json::Value>>(),
            )
        };
        
        let vector = vectordb_common::types::Vector {
            id: vector_id,
            data: vector_proto.data,
            metadata,
        };
        
        match self.store.insert(&req.collection_name, &vector).await {
            Ok(()) => {
                Ok(Response::new(InsertResponse {
                    success: true,
                    message: "Vector inserted successfully".to_string(),
                }))
            }
            Err(e) => {
                error!("Failed to insert vector: {}", e);
                Ok(Response::new(InsertResponse {
                    success: false,
                    message: e.to_string(),
                }))
            }
        }
    }
    
    #[instrument(skip(self))]
    async fn batch_insert(
        &self,
        request: Request<BatchInsertRequest>,
    ) -> Result<Response<BatchInsertResponse>, Status> {
        let req = request.into_inner();
        
        let mut vectors = Vec::new();
        
        for vector_proto in req.vectors {
            let vector_id = Uuid::parse_str(&vector_proto.id)
                .map_err(|_| Status::invalid_argument("Invalid vector ID format"))?;
            
            let metadata = if vector_proto.metadata.is_empty() {
                None
            } else {
                Some(
                    vector_proto
                        .metadata
                        .into_iter()
                        .map(|(k, v)| (k, serde_json::Value::String(v)))
                        .collect::<HashMap<String, serde_json::Value>>(),
                )
            };
            
            vectors.push(vectordb_common::types::Vector {
                id: vector_id,
                data: vector_proto.data,
                metadata,
            });
        }
        
        match self.store.batch_insert(&req.collection_name, &vectors).await {
            Ok(()) => {
                Ok(Response::new(BatchInsertResponse {
                    success: true,
                    message: "Vectors inserted successfully".to_string(),
                    inserted_count: vectors.len() as u32,
                }))
            }
            Err(e) => {
                error!("Failed to batch insert vectors: {}", e);
                Ok(Response::new(BatchInsertResponse {
                    success: false,
                    message: e.to_string(),
                    inserted_count: 0,
                }))
            }
        }
    }
    
    #[instrument(skip(self))]
    async fn delete(
        &self,
        request: Request<DeleteRequest>,
    ) -> Result<Response<DeleteResponse>, Status> {
        let req = request.into_inner();
        
        let vector_id = Uuid::parse_str(&req.vector_id)
            .map_err(|_| Status::invalid_argument("Invalid vector ID format"))?;
        
        match self.store.delete(&req.collection_name, &vector_id).await {
            Ok(deleted) => {
                if deleted {
                    Ok(Response::new(DeleteResponse {
                        success: true,
                        message: "Vector deleted successfully".to_string(),
                    }))
                } else {
                    Ok(Response::new(DeleteResponse {
                        success: false,
                        message: "Vector not found".to_string(),
                    }))
                }
            }
            Err(e) => {
                error!("Failed to delete vector: {}", e);
                Ok(Response::new(DeleteResponse {
                    success: false,
                    message: e.to_string(),
                }))
            }
        }
    }
    
    #[instrument(skip(self))]
    async fn query(
        &self,
        request: Request<QueryRequest>,
    ) -> Result<Response<QueryResponse>, Status> {
        let start_time = std::time::Instant::now();
        let req = request.into_inner();
        
        // TODO: Parse proto filter map into Filter type
        // For now, simple filters are not supported via gRPC
        let filter = None;

        let query_request = vectordb_common::types::QueryRequest {
            collection: req.collection_name,
            vector: req.query_vector,
            limit: req.limit as usize,
            ef_search: req.ef_search.map(|ef| ef as usize),
            filter,
        };
        
        match self.store.query(&query_request).await {
            Ok(results) => {
                let query_time_ms = start_time.elapsed().as_millis() as u64;
                
                let proto_results: Vec<QueryResult> = results
                    .into_iter()
                    .map(|r| QueryResult {
                        id: r.id.to_string(),
                        distance: r.distance,
                        metadata: r.metadata.map_or(HashMap::new(), |meta| {
                            meta.into_iter()
                                .map(|(k, v)| (k, v.to_string()))
                                .collect()
                        }),
                    })
                    .collect();
                
                Ok(Response::new(QueryResponse {
                    results: proto_results,
                    query_time_ms,
                }))
            }
            Err(e) => {
                error!("Failed to query vectors: {}", e);
                Err(Status::internal(e.to_string()))
            }
        }
    }
    
    #[instrument(skip(self))]
    async fn update(
        &self,
        request: Request<UpdateRequest>,
    ) -> Result<Response<UpdateResponse>, Status> {
        let req = request.into_inner();
        
        let vector_proto = req.vector.ok_or_else(|| {
            Status::invalid_argument("Vector is required")
        })?;
        
        let vector_id = Uuid::parse_str(&vector_proto.id)
            .map_err(|_| Status::invalid_argument("Invalid vector ID format"))?;
        
        let metadata = if vector_proto.metadata.is_empty() {
            None
        } else {
            Some(
                vector_proto
                    .metadata
                    .into_iter()
                    .map(|(k, v)| (k, serde_json::Value::String(v)))
                    .collect::<HashMap<String, serde_json::Value>>(),
            )
        };
        
        let vector = vectordb_common::types::Vector {
            id: vector_id,
            data: vector_proto.data,
            metadata,
        };
        
        match self.store.update(&req.collection_name, &vector).await {
            Ok(()) => {
                Ok(Response::new(UpdateResponse {
                    success: true,
                    message: "Vector updated successfully".to_string(),
                }))
            }
            Err(e) => {
                error!("Failed to update vector: {}", e);
                Ok(Response::new(UpdateResponse {
                    success: false,
                    message: e.to_string(),
                }))
            }
        }
    }
    
    #[instrument(skip(self))]
    async fn get_stats(
        &self,
        _request: Request<GetStatsRequest>,
    ) -> Result<Response<GetStatsResponse>, Status> {
        match self.store.get_server_stats().await {
            Ok(stats) => {
                let proto_stats = vectordb_proto::ServerStats {
                    total_vectors: stats.total_vectors,
                    total_collections: stats.total_collections,
                    memory_usage: stats.memory_usage,
                    disk_usage: stats.disk_usage,
                    uptime_seconds: stats.uptime_seconds,
                };
                
                Ok(Response::new(GetStatsResponse {
                    stats: Some(proto_stats),
                }))
            }
            Err(e) => {
                error!("Failed to get server stats: {}", e);
                Err(Status::internal(e.to_string()))
            }
        }
    }
    
    #[instrument(skip(self))]
    async fn health(
        &self,
        _request: Request<HealthRequest>,
    ) -> Result<Response<HealthResponse>, Status> {
        Ok(Response::new(HealthResponse {
            healthy: true,
            status: "OK".to_string(),
        }))
    }

    // Advanced search operations (TODO: implement)
    async fn recommend(
        &self,
        _request: Request<vectordb_proto::RecommendRequest>,
    ) -> Result<Response<vectordb_proto::RecommendResponse>, Status> {
        Err(Status::unimplemented("Recommend API not yet implemented"))
    }

    async fn discover(
        &self,
        _request: Request<vectordb_proto::DiscoverRequest>,
    ) -> Result<Response<vectordb_proto::DiscoverResponse>, Status> {
        Err(Status::unimplemented("Discover API not yet implemented"))
    }

    async fn scroll(
        &self,
        _request: Request<vectordb_proto::ScrollRequest>,
    ) -> Result<Response<vectordb_proto::ScrollResponse>, Status> {
        Err(Status::unimplemented("Scroll API not yet implemented"))
    }

    async fn count(
        &self,
        _request: Request<vectordb_proto::CountRequest>,
    ) -> Result<Response<vectordb_proto::CountResponse>, Status> {
        Err(Status::unimplemented("Count API not yet implemented"))
    }

    async fn batch_search(
        &self,
        _request: Request<vectordb_proto::BatchSearchRequest>,
    ) -> Result<Response<vectordb_proto::BatchSearchResponse>, Status> {
        Err(Status::unimplemented("Batch search API not yet implemented"))
    }

    // Snapshot operations (TODO: implement)
    async fn create_snapshot(
        &self,
        _request: Request<vectordb_proto::CreateSnapshotRequest>,
    ) -> Result<Response<vectordb_proto::CreateSnapshotResponse>, Status> {
        Err(Status::unimplemented("Create snapshot not yet implemented"))
    }

    async fn list_snapshots(
        &self,
        _request: Request<vectordb_proto::ListSnapshotsRequest>,
    ) -> Result<Response<vectordb_proto::ListSnapshotsResponse>, Status> {
        Err(Status::unimplemented("List snapshots not yet implemented"))
    }

    async fn get_snapshot(
        &self,
        _request: Request<vectordb_proto::GetSnapshotRequest>,
    ) -> Result<Response<vectordb_proto::GetSnapshotResponse>, Status> {
        Err(Status::unimplemented("Get snapshot not yet implemented"))
    }

    async fn delete_snapshot(
        &self,
        _request: Request<vectordb_proto::DeleteSnapshotRequest>,
    ) -> Result<Response<vectordb_proto::DeleteSnapshotResponse>, Status> {
        Err(Status::unimplemented("Delete snapshot not yet implemented"))
    }

    async fn restore_snapshot(
        &self,
        _request: Request<vectordb_proto::RestoreSnapshotRequest>,
    ) -> Result<Response<vectordb_proto::RestoreSnapshotResponse>, Status> {
        Err(Status::unimplemented("Restore snapshot not yet implemented"))
    }
}

/// Start the gRPC server
pub async fn start_grpc_server(addr: SocketAddr, store: Arc<VectorStore>) -> anyhow::Result<()> {
    use tonic::transport::Server;
    
    let service = VectorDbService::new(store);
    
    info!("Starting gRPC server on {}", addr);
    
    Server::builder()
        .add_service(VectorDbServer::new(service))
        .serve(addr)
        .await?;
    
    Ok(())
}