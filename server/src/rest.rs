use vectordb_vectorstore::VectorStore;
use vectordb_common::types::*;
use std::sync::Arc;
use std::collections::HashMap;
use std::time::Duration;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post, delete, put},
    Router,
};
use serde::{Deserialize, Serialize};
use tracing::{info, error, instrument};
use uuid::Uuid;
use std::net::SocketAddr;
use tokio::time::timeout;

/// REST API response wrapper
#[derive(Serialize)]
#[serde(bound(serialize = "T: Serialize"))]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
        }
    }
}

/// Collection creation request
#[derive(Deserialize, Debug)]
struct CreateCollectionRequest {
    name: String,
    dimension: usize,
    distance_metric: DistanceMetric,
    vector_type: VectorType,
    index_config: Option<IndexConfig>,
    quantization: Option<vectordb_common::quantization::QuantizationConfig>,
}

/// Collection creation response
#[derive(Serialize, Debug)]
struct CreateCollectionResponse {
    name: String,
    message: String,
}

/// Collection deletion response
#[derive(Serialize, Debug)]
struct DeleteCollectionResponse {
    name: String,
    message: String,
}

/// Vector update response
#[derive(Serialize, Debug)]
struct UpdateVectorResponse {
    id: String,
    message: String,
}

/// Vector insertion request
#[derive(Deserialize, Debug)]
struct InsertVectorRequest {
    id: Option<String>,
    data: Vec<f32>,
    metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Batch vector insertion request
#[derive(Deserialize, Debug)]
struct BatchInsertRequest {
    vectors: Vec<InsertVectorRequest>,
}

/// Batch upsert response
#[derive(Serialize, Debug)]
struct BatchUpsertResponse {
    upserted_count: usize,
    ids: Vec<String>,
}

/// Batch delete request
#[derive(Deserialize, Debug)]
struct BatchDeleteRequest {
    ids: Vec<String>,
}

/// Batch delete response
#[derive(Serialize, Debug)]
struct BatchDeleteResponse {
    deleted_count: usize,
}

/// Query request
#[derive(Deserialize, Debug)]
struct QueryVectorsRequest {
    #[serde(alias = "query_vector")]
    vector: Vec<f32>,
    limit: Option<usize>,
    ef_search: Option<usize>,
    filter: Option<HashMap<String, serde_json::Value>>,
}

/// Query parameters for search
#[derive(Deserialize, Debug)]
struct QueryParams {
    limit: Option<usize>,
    ef_search: Option<usize>,
}

type AppState = Arc<VectorStore>;

/// Create collection
#[instrument(skip(state))]
async fn create_collection(
    State(state): State<AppState>,
    Json(payload): Json<CreateCollectionRequest>,
) -> Result<Json<ApiResponse<CreateCollectionResponse>>, StatusCode> {
    let config = CollectionConfig {
        name: payload.name.clone(),
        dimension: payload.dimension,
        distance_metric: payload.distance_metric,
        vector_type: payload.vector_type,
        index_config: payload.index_config.unwrap_or_default(),
        quantization: payload.quantization,
    };

    match state.create_collection(&config).await {
        Ok(()) => Ok(Json(ApiResponse::success(CreateCollectionResponse {
            name: payload.name,
            message: "Collection created successfully".to_string(),
        }))),
        Err(e) => {
            error!("Failed to create collection: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}

/// List collections
#[instrument(skip(state))]
async fn list_collections(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<String>>>, StatusCode> {
    let collections = state.list_collections();
    Ok(Json(ApiResponse::success(collections)))
}

/// Get collection info
#[instrument(skip(state))]
async fn get_collection_info(
    State(state): State<AppState>,
    Path(collection_name): Path<String>,
) -> Result<Json<ApiResponse<(CollectionConfig, CollectionStats)>>, StatusCode> {
    let config = match state.get_collection_config(&collection_name) {
        Ok(Some(config)) => config,
        Ok(None) => return Ok(Json(ApiResponse::error("Collection not found".to_string()))),
        Err(e) => return Ok(Json(ApiResponse::error(e.to_string()))),
    };
    
    let stats = match state.get_collection_stats(&collection_name).await {
        Ok(Some(stats)) => stats,
        Ok(None) => return Ok(Json(ApiResponse::error("Collection not found".to_string()))),
        Err(e) => return Ok(Json(ApiResponse::error(e.to_string()))),
    };
    
    Ok(Json(ApiResponse::success((config, stats))))
}

/// Delete collection
#[instrument(skip(state))]
async fn delete_collection(
    State(state): State<AppState>,
    Path(collection_name): Path<String>,
) -> Result<Json<ApiResponse<DeleteCollectionResponse>>, StatusCode> {
    match state.delete_collection(&collection_name).await {
        Ok(()) => Ok(Json(ApiResponse::success(DeleteCollectionResponse {
            name: collection_name.clone(),
            message: "Collection deleted successfully".to_string(),
        }))),
        Err(e) => {
            error!("Failed to delete collection: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}

/// Insert vector
#[instrument(skip(state))]
async fn insert_vector(
    State(state): State<AppState>,
    Path(collection_name): Path<String>,
    Json(payload): Json<InsertVectorRequest>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    let vector_id = if let Some(id_str) = payload.id {
        Uuid::parse_str(&id_str)
            .map_err(|_| StatusCode::BAD_REQUEST)?
    } else {
        Uuid::new_v4()
    };
    
    let vector = Vector {
        id: vector_id,
        data: payload.data,
        metadata: payload.metadata,
    };

    // Add timeout to prevent indefinite hangs (30 seconds default)
    let insert_timeout = Duration::from_secs(30);
    match timeout(insert_timeout, state.insert(&collection_name, &vector)).await {
        Ok(Ok(())) => Ok(Json(ApiResponse::success(vector_id.to_string()))),
        Ok(Err(e)) => {
            error!("Failed to insert vector: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
        Err(_) => {
            error!("Vector insertion timed out after {:?}", insert_timeout);
            Ok(Json(ApiResponse::error(format!(
                "Operation timed out after {:?}. The server may be under heavy load.",
                insert_timeout
            ))))
        }
    }
}

/// Batch insert vectors
#[instrument(skip(state))]
async fn batch_insert_vectors(
    State(state): State<AppState>,
    Path(collection_name): Path<String>,
    Json(payload): Json<BatchInsertRequest>,
) -> Result<Json<ApiResponse<Vec<String>>>, StatusCode> {
    let mut vectors = Vec::new();
    let mut vector_ids = Vec::new();
    
    for vector_req in payload.vectors {
        let vector_id = if let Some(id_str) = vector_req.id {
            Uuid::parse_str(&id_str)
                .map_err(|_| StatusCode::BAD_REQUEST)?
        } else {
            Uuid::new_v4()
        };
        
        vector_ids.push(vector_id.to_string());
        vectors.push(Vector {
            id: vector_id,
            data: vector_req.data,
            metadata: vector_req.metadata,
        });
    }

    // Add timeout for batch insert (60 seconds for larger batches)
    let batch_timeout = Duration::from_secs(60);
    match timeout(batch_timeout, state.batch_insert(&collection_name, &vectors)).await {
        Ok(Ok(())) => Ok(Json(ApiResponse::success(vector_ids))),
        Ok(Err(e)) => {
            error!("Failed to batch insert vectors: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
        Err(_) => {
            error!("Batch insert timed out after {:?} for {} vectors", batch_timeout, vectors.len());
            Ok(Json(ApiResponse::error(format!(
                "Batch operation timed out after {:?} while inserting {} vectors. Try reducing batch size.",
                batch_timeout,
                vectors.len()
            ))))
        }
    }
}

/// Batch upsert vectors (update if exists, insert if not)
#[instrument(skip(state))]
async fn batch_upsert_vectors(
    State(state): State<AppState>,
    Path(collection_name): Path<String>,
    Json(payload): Json<BatchInsertRequest>,
) -> Result<Json<ApiResponse<BatchUpsertResponse>>, StatusCode> {
    let mut vectors = Vec::new();
    let mut vector_ids = Vec::new();

    for vector_req in payload.vectors {
        let vector_id = if let Some(id_str) = vector_req.id {
            Uuid::parse_str(&id_str)
                .map_err(|_| StatusCode::BAD_REQUEST)?
        } else {
            Uuid::new_v4()
        };

        vector_ids.push(vector_id.to_string());
        vectors.push(Vector {
            id: vector_id,
            data: vector_req.data,
            metadata: vector_req.metadata,
        });
    }

    let batch_timeout = Duration::from_secs(60);
    match timeout(batch_timeout, state.batch_upsert(&collection_name, &vectors)).await {
        Ok(Ok(count)) => Ok(Json(ApiResponse::success(BatchUpsertResponse {
            upserted_count: count,
            ids: vector_ids,
        }))),
        Ok(Err(e)) => {
            error!("Failed to batch upsert vectors: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
        Err(_) => {
            error!("Batch upsert timed out after {:?} for {} vectors", batch_timeout, vectors.len());
            Ok(Json(ApiResponse::error(format!(
                "Batch operation timed out after {:?} while upserting {} vectors. Try reducing batch size.",
                batch_timeout,
                vectors.len()
            ))))
        }
    }
}

/// Batch delete vectors
#[instrument(skip(state))]
async fn batch_delete_vectors(
    State(state): State<AppState>,
    Path(collection_name): Path<String>,
    Json(payload): Json<BatchDeleteRequest>,
) -> Result<Json<ApiResponse<BatchDeleteResponse>>, StatusCode> {
    let mut ids = Vec::new();

    for id_str in &payload.ids {
        let uuid = Uuid::parse_str(id_str)
            .map_err(|_| StatusCode::BAD_REQUEST)?;
        ids.push(uuid);
    }

    let batch_timeout = Duration::from_secs(60);
    match timeout(batch_timeout, state.batch_delete(&collection_name, &ids)).await {
        Ok(Ok(deleted_count)) => Ok(Json(ApiResponse::success(BatchDeleteResponse {
            deleted_count,
        }))),
        Ok(Err(e)) => {
            error!("Failed to batch delete vectors: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
        Err(_) => {
            error!("Batch delete timed out after {:?} for {} vectors", batch_timeout, ids.len());
            Ok(Json(ApiResponse::error(format!(
                "Batch operation timed out after {:?} while deleting {} vectors. Try reducing batch size.",
                batch_timeout,
                ids.len()
            ))))
        }
    }
}

/// Query vectors
#[instrument(skip(state))]
async fn query_vectors(
    State(state): State<AppState>,
    Path(collection_name): Path<String>,
    Query(params): Query<QueryParams>,
    Json(payload): Json<QueryVectorsRequest>,
) -> Result<Json<ApiResponse<Vec<QueryResult>>>, StatusCode> {
    let query_request = QueryRequest {
        collection: collection_name,
        vector: payload.vector,
        limit: payload.limit.or(params.limit).unwrap_or(10),
        ef_search: payload.ef_search.or(params.ef_search),
        filter: payload.filter,
    };
    
    match state.query(&query_request).await {
        Ok(results) => Ok(Json(ApiResponse::success(results))),
        Err(e) => {
            error!("Failed to query vectors: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}

/// Get vector by ID
#[instrument(skip(state))]
async fn get_vector(
    State(state): State<AppState>,
    Path((collection_name, vector_id)): Path<(String, String)>,
) -> Result<Json<ApiResponse<Option<Vector>>>, StatusCode> {
    let uuid = Uuid::parse_str(&vector_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    match state.get(&collection_name, &uuid).await {
        Ok(vector) => Ok(Json(ApiResponse::success(vector))),
        Err(e) => {
            error!("Failed to get vector: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}

/// Delete vector
#[instrument(skip(state))]
async fn delete_vector(
    State(state): State<AppState>,
    Path((collection_name, vector_id)): Path<(String, String)>,
) -> Result<Json<ApiResponse<bool>>, StatusCode> {
    let uuid = Uuid::parse_str(&vector_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    match state.delete(&collection_name, &uuid).await {
        Ok(deleted) => Ok(Json(ApiResponse::success(deleted))),
        Err(e) => {
            error!("Failed to delete vector: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}

/// Update vector
#[instrument(skip(state))]
async fn update_vector(
    State(state): State<AppState>,
    Path((collection_name, vector_id)): Path<(String, String)>,
    Json(payload): Json<InsertVectorRequest>,
) -> Result<Json<ApiResponse<UpdateVectorResponse>>, StatusCode> {
    let uuid = Uuid::parse_str(&vector_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let vector = Vector {
        id: uuid,
        data: payload.data,
        metadata: payload.metadata,
    };

    match state.update(&collection_name, &vector).await {
        Ok(()) => Ok(Json(ApiResponse::success(UpdateVectorResponse {
            id: vector_id.clone(),
            message: "Vector updated successfully".to_string(),
        }))),
        Err(e) => {
            error!("Failed to update vector: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}

/// Get server stats
#[instrument(skip(state))]
async fn get_stats(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<vectordb_vectorstore::ServerStats>>, StatusCode> {
    match state.get_server_stats().await {
        Ok(stats) => Ok(Json(ApiResponse::success(stats))),
        Err(e) => {
            error!("Failed to get server stats: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}

/// Health check (backward compatibility - redirects to simple health)
#[instrument]
async fn health() -> Result<Json<ApiResponse<String>>, StatusCode> {
    Ok(Json(ApiResponse::success("OK".to_string())))
}

// === RECOVERY ENDPOINTS ===

#[derive(Serialize, Debug)]
struct BackupResponse {
    collection: String,
    backup_path: String,
    message: String,
}

#[derive(Serialize, Debug)]
struct DeletedCollectionInfo {
    name: String,
    path: String,
    deleted_timestamp: u64,
    age_hours: f64,
}

#[derive(Deserialize, Debug)]
struct RestoreRequest {
    backup_path: String,
    new_name: Option<String>,
}

#[derive(Deserialize, Debug)]
struct ImportRequest {
    orphaned_path: String,
    collection_name: String,
    dimension: usize,
    distance_metric: DistanceMetric,
    vector_type: VectorType,
}

#[derive(Deserialize, Debug)]
struct CleanupRequest {
    retention_hours: Option<u64>,
}

/// Create backup of a collection
#[instrument(skip(state))]
async fn backup_collection(
    State(state): State<AppState>,
    Path(collection_name): Path<String>,
) -> Result<Json<ApiResponse<BackupResponse>>, StatusCode> {
    match state.backup_collection(&collection_name).await {
        Ok(backup_path) => Ok(Json(ApiResponse::success(BackupResponse {
            collection: collection_name,
            backup_path: backup_path.display().to_string(),
            message: "Backup created successfully".to_string(),
        }))),
        Err(e) => {
            error!("Failed to backup collection: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}

/// List soft-deleted collections
#[instrument(skip(state))]
async fn list_deleted_collections(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<DeletedCollectionInfo>>>, StatusCode> {
    match state.list_deleted_collections().await {
        Ok(deleted) => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();

            let info: Vec<DeletedCollectionInfo> = deleted
                .into_iter()
                .map(|(name, path, timestamp)| {
                    let age_hours = (now.saturating_sub(timestamp)) as f64 / 3600.0;
                    DeletedCollectionInfo {
                        name,
                        path: path.display().to_string(),
                        deleted_timestamp: timestamp,
                        age_hours,
                    }
                })
                .collect();

            Ok(Json(ApiResponse::success(info)))
        }
        Err(e) => {
            error!("Failed to list deleted collections: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}

/// Restore a collection from backup or soft-delete
#[instrument(skip(state))]
async fn restore_collection(
    State(state): State<AppState>,
    Json(payload): Json<RestoreRequest>,
) -> Result<Json<ApiResponse<CreateCollectionResponse>>, StatusCode> {
    let backup_path = std::path::PathBuf::from(&payload.backup_path);

    match state.restore_collection(&backup_path, payload.new_name.as_deref()).await {
        Ok(restored_name) => Ok(Json(ApiResponse::success(CreateCollectionResponse {
            name: restored_name.clone(),
            message: format!("Collection '{}' restored successfully", restored_name),
        }))),
        Err(e) => {
            error!("Failed to restore collection: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}

/// Import orphaned collection data
#[instrument(skip(state))]
async fn import_orphaned_collection(
    State(state): State<AppState>,
    Json(payload): Json<ImportRequest>,
) -> Result<Json<ApiResponse<CreateCollectionResponse>>, StatusCode> {
    let orphaned_path = std::path::PathBuf::from(&payload.orphaned_path);

    let config = CollectionConfig {
        name: payload.collection_name.clone(),
        dimension: payload.dimension,
        distance_metric: payload.distance_metric,
        vector_type: payload.vector_type,
        index_config: IndexConfig::default(),
    };

    match state.import_orphaned_collection(&orphaned_path, &payload.collection_name, &config).await {
        Ok(()) => Ok(Json(ApiResponse::success(CreateCollectionResponse {
            name: payload.collection_name.clone(),
            message: format!("Collection '{}' imported successfully from orphaned data", payload.collection_name),
        }))),
        Err(e) => {
            error!("Failed to import orphaned collection: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}

/// Hard delete a collection (permanent, no recovery)
#[instrument(skip(state))]
async fn hard_delete_collection(
    State(state): State<AppState>,
    Path(collection_name): Path<String>,
) -> Result<Json<ApiResponse<DeleteCollectionResponse>>, StatusCode> {
    match state.hard_delete_collection(&collection_name).await {
        Ok(()) => Ok(Json(ApiResponse::success(DeleteCollectionResponse {
            name: collection_name.clone(),
            message: "Collection permanently deleted (no recovery possible)".to_string(),
        }))),
        Err(e) => {
            error!("Failed to hard delete collection: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}

/// Cleanup old soft-deleted collections
#[instrument(skip(state))]
async fn cleanup_old_deleted(
    State(state): State<AppState>,
    Json(payload): Json<CleanupRequest>,
) -> Result<Json<ApiResponse<Vec<String>>>, StatusCode> {
    let retention_hours = payload.retention_hours.unwrap_or(24);

    match state.cleanup_old_deleted(retention_hours).await {
        Ok(cleaned) => Ok(Json(ApiResponse::success(cleaned))),
        Err(e) => {
            error!("Failed to cleanup old deleted collections: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}

// ==================== Advanced Search Handlers ====================

/// Recommend points based on positive and negative examples
#[instrument(skip(state))]
async fn recommend_points(
    State(state): State<AppState>,
    Path(collection): Path<String>,
    Json(mut request): Json<vectordb_common::RecommendRequest>,
) -> Result<Json<ApiResponse<Vec<QueryResult>>>, StatusCode> {
    request.collection = collection;

    match state.recommend(&request).await {
        Ok(results) => Ok(Json(ApiResponse::success(results))),
        Err(e) => {
            error!("Failed to execute recommend: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}

/// Discovery search - find vectors in direction of context
#[instrument(skip(state))]
async fn discover_points(
    State(state): State<AppState>,
    Path(collection): Path<String>,
    Json(mut request): Json<vectordb_common::DiscoveryRequest>,
) -> Result<Json<ApiResponse<Vec<QueryResult>>>, StatusCode> {
    request.collection = collection;

    match state.discover(&request).await {
        Ok(results) => Ok(Json(ApiResponse::success(results))),
        Err(e) => {
            error!("Failed to execute discover: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}

/// Scroll through points with pagination
#[instrument(skip(state))]
async fn scroll_points(
    State(state): State<AppState>,
    Path(collection): Path<String>,
    Json(mut request): Json<vectordb_common::ScrollRequest>,
) -> Result<Json<ApiResponse<vectordb_common::ScrollResponse>>, StatusCode> {
    request.collection = collection;

    match state.scroll(&request).await {
        Ok(response) => Ok(Json(ApiResponse::success(response))),
        Err(e) => {
            error!("Failed to execute scroll: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}

/// Count points matching filter
#[instrument(skip(state))]
async fn count_points(
    State(state): State<AppState>,
    Path(collection): Path<String>,
    Json(mut request): Json<vectordb_common::CountRequest>,
) -> Result<Json<ApiResponse<vectordb_common::CountResponse>>, StatusCode> {
    request.collection = collection;

    match state.count(&request).await {
        Ok(response) => Ok(Json(ApiResponse::success(response))),
        Err(e) => {
            error!("Failed to execute count: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}

/// Batch search - multiple queries in one request
#[instrument(skip(state))]
async fn batch_search_points(
    State(state): State<AppState>,
    Path(collection): Path<String>,
    Json(mut request): Json<vectordb_common::BatchSearchRequest>,
) -> Result<Json<ApiResponse<Vec<Vec<QueryResult>>>>, StatusCode> {
    request.collection = collection;

    match state.batch_search(&request).await {
        Ok(results) => Ok(Json(ApiResponse::success(results))),
        Err(e) => {
            error!("Failed to execute batch search: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}

// ==================== Snapshot Handlers ====================

#[derive(Serialize, Debug)]
struct SnapshotCreatedResponse {
    snapshot_name: String,
    collection: String,
    size_bytes: u64,
    message: String,
}

/// Create a snapshot of a collection
#[instrument(skip(state))]
async fn create_snapshot(
    State(state): State<AppState>,
    Path(collection): Path<String>,
) -> Result<Json<ApiResponse<SnapshotCreatedResponse>>, StatusCode> {
    match state.create_snapshot(&collection).await {
        Ok(metadata) => Ok(Json(ApiResponse::success(SnapshotCreatedResponse {
            snapshot_name: metadata.name.clone(),
            collection: metadata.collection,
            size_bytes: metadata.size_bytes,
            message: "Snapshot created successfully".to_string(),
        }))),
        Err(e) => {
            error!("Failed to create snapshot: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}

/// List all snapshots
#[instrument(skip(state))]
async fn list_snapshots_handler(
    State(state): State<AppState>,
    Path(_collection): Path<String>,
) -> Result<Json<ApiResponse<Vec<vectordb_storage::SnapshotMetadata>>>, StatusCode> {
    match state.list_snapshots() {
        Ok(snapshots) => Ok(Json(ApiResponse::success(snapshots))),
        Err(e) => {
            error!("Failed to list snapshots: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}

/// Get snapshot info
#[instrument(skip(state))]
async fn get_snapshot_handler(
    State(state): State<AppState>,
    Path((_collection, snapshot_id)): Path<(String, String)>,
) -> Result<Json<ApiResponse<vectordb_storage::SnapshotMetadata>>, StatusCode> {
    match state.get_snapshot(&snapshot_id) {
        Ok(snapshot) => Ok(Json(ApiResponse::success(snapshot))),
        Err(e) => {
            error!("Failed to get snapshot: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}

/// Delete a snapshot
#[instrument(skip(state))]
async fn delete_snapshot_handler(
    State(state): State<AppState>,
    Path((_collection, snapshot_id)): Path<(String, String)>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    match state.delete_snapshot(&snapshot_id) {
        Ok(()) => Ok(Json(ApiResponse::success(format!(
            "Snapshot '{}' deleted successfully",
            snapshot_id
        )))),
        Err(e) => {
            error!("Failed to delete snapshot: {}", e);
            Ok(Json(ApiResponse::error(e.to_string())))
        }
    }
}

/// Create REST API router
pub fn create_router(state: AppState) -> Router {
    use crate::health;

    Router::new()
        // Collection management
        .route("/collections", post(create_collection))
        .route("/collections", get(list_collections))
        .route("/collections/:collection", get(get_collection_info))
        .route("/collections/:collection", delete(delete_collection))

        // Recovery operations
        .route("/collections/:collection/backup", post(backup_collection))
        .route("/collections/:collection/hard-delete", delete(hard_delete_collection))
        .route("/collections/deleted", get(list_deleted_collections))
        .route("/collections/restore", post(restore_collection))
        .route("/collections/import", post(import_orphaned_collection))
        .route("/collections/cleanup", post(cleanup_old_deleted))

        // Vector operations
        .route("/collections/:collection/vectors", post(insert_vector))
        .route("/collections/:collection/vectors/batch", post(batch_insert_vectors))
        .route("/collections/:collection/vectors/upsert", post(batch_upsert_vectors))
        .route("/collections/:collection/vectors/batch-delete", post(batch_delete_vectors))
        .route("/collections/:collection/search", post(query_vectors))
        .route("/collections/:collection/vectors/:vector_id", get(get_vector))
        .route("/collections/:collection/vectors/:vector_id", put(update_vector))
        .route("/collections/:collection/vectors/:vector_id", delete(delete_vector))

        // Advanced search operations
        .route("/collections/:collection/points/recommend", post(recommend_points))
        .route("/collections/:collection/points/discover", post(discover_points))
        .route("/collections/:collection/points/scroll", post(scroll_points))
        .route("/collections/:collection/points/count", post(count_points))
        .route("/collections/:collection/points/search/batch", post(batch_search_points))

        // Snapshot operations
        .route("/collections/:collection/snapshots", post(create_snapshot))
        .route("/collections/:collection/snapshots", get(list_snapshots_handler))
        .route("/collections/:collection/snapshots/:snapshot_id", get(get_snapshot_handler))
        .route("/collections/:collection/snapshots/:snapshot_id", delete(delete_snapshot_handler))

        // Server operations
        .route("/stats", get(get_stats))

        // Health check endpoints
        .route("/health", get(health))                           // Backward compatibility
        .route("/health/live", get(health::health_liveness))     // Kubernetes liveness probe
        .route("/health/ready", get(health::health_readiness))   // Kubernetes readiness probe
        .route("/ready", get(health::health_readiness))          // Short alias for readiness
        .route("/health/check", get(health::health_check))       // Deep health check

        .with_state(state)
}

/// Start the REST server
pub async fn start_rest_server(addr: SocketAddr, store: Arc<VectorStore>) -> anyhow::Result<()> {
    let app = create_router(store);
    
    info!("Starting REST server on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}