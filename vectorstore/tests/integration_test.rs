use vectordb_common::types::*;
use vectordb_vectorstore::VectorStore;
use uuid::Uuid;

#[tokio::test]
async fn test_collection_lifecycle() {
    let temp_dir = tempfile::tempdir().unwrap();
    let store = VectorStore::new(temp_dir.path()).await.unwrap();

    // Create collection
    let config = CollectionConfig {
        name: "test_collection".to_string(),
        dimension: 128,
        distance_metric: DistanceMetric::Cosine,
        vector_type: VectorType::Float32,
        index_config: IndexConfig::default(),
        quantization: None,
    };

    store.create_collection(&config).await.unwrap();

    // Verify collection exists
    let collections = store.list_collections();
    assert_eq!(collections.len(), 1);
    assert_eq!(collections[0], "test_collection");

    // Get collection info
    let retrieved_config = store
        .get_collection_config("test_collection")
        .unwrap()
        .unwrap();
    assert_eq!(retrieved_config.dimension, 128);

    // Delete collection
    store.delete_collection("test_collection").await.unwrap();
    let collections = store.list_collections();
    assert_eq!(collections.len(), 0);
}

#[tokio::test]
async fn test_vector_operations() {
    let temp_dir = tempfile::tempdir().unwrap();
    let store = VectorStore::new(temp_dir.path()).await.unwrap();

    let config = CollectionConfig {
        name: "vectors".to_string(),
        dimension: 3,
        distance_metric: DistanceMetric::Euclidean,
        vector_type: VectorType::Float32,
        index_config: IndexConfig::default(),
        quantization: None,
    };

    store.create_collection(&config).await.unwrap();

    // Insert vector
    let vec_id = Uuid::new_v4();
    let vector = Vector {
        id: vec_id,
        data: vec![1.0, 2.0, 3.0],
        metadata: Some(
            vec![("key".to_string(), serde_json::Value::String("value".to_string()))]
                .into_iter()
                .collect(),
        ),
    };

    store.insert("vectors", &vector).await.unwrap();

    // Get vector
    let retrieved = store.get("vectors", &vec_id).await.unwrap().unwrap();
    assert_eq!(retrieved.id, vec_id);
    assert_eq!(retrieved.data, vec![1.0, 2.0, 3.0]);

    // Update vector
    let mut updated_vector = retrieved.clone();
    updated_vector.data = vec![4.0, 5.0, 6.0];
    store.update("vectors", &updated_vector).await.unwrap();

    let retrieved_updated = store.get("vectors", &vec_id).await.unwrap().unwrap();
    assert_eq!(retrieved_updated.data, vec![4.0, 5.0, 6.0]);

    // Delete vector
    let deleted = store.delete("vectors", &vec_id).await.unwrap();
    assert!(deleted);

    let after_delete = store.get("vectors", &vec_id).await.unwrap();
    assert!(after_delete.is_none());
}

#[tokio::test]
async fn test_batch_operations() {
    let temp_dir = tempfile::tempdir().unwrap();
    let store = VectorStore::new(temp_dir.path()).await.unwrap();

    let config = CollectionConfig {
        name: "batch_test".to_string(),
        dimension: 2,
        distance_metric: DistanceMetric::Cosine,
        vector_type: VectorType::Float32,
        index_config: IndexConfig::default(),
        quantization: None,
    };

    store.create_collection(&config).await.unwrap();

    // Batch insert
    let vectors: Vec<Vector> = (0..10)
        .map(|i| Vector {
            id: Uuid::new_v4(),
            data: vec![i as f32, (i + 1) as f32],
            metadata: None,
        })
        .collect();

    store.batch_insert("batch_test", &vectors).await.unwrap();

    // Batch upsert (some new, some existing)
    let mut upsert_vectors = vectors[0..5].to_vec();
    for v in &mut upsert_vectors {
        v.data[0] += 10.0; // Modify existing
    }

    // Add new vectors
    for i in 10..15 {
        upsert_vectors.push(Vector {
            id: Uuid::new_v4(),
            data: vec![i as f32, (i + 1) as f32],
            metadata: None,
        });
    }

    let upserted = store
        .batch_upsert("batch_test", &upsert_vectors)
        .await
        .unwrap();
    assert_eq!(upserted, 10); // 5 updates + 5 inserts

    // Batch delete
    let ids_to_delete: Vec<Uuid> = vectors[0..3].iter().map(|v| v.id).collect();
    let deleted = store
        .batch_delete("batch_test", &ids_to_delete)
        .await
        .unwrap();
    assert_eq!(deleted, 3);
}

#[tokio::test]
async fn test_search_query() {
    let temp_dir = tempfile::tempdir().unwrap();
    let store = VectorStore::new(temp_dir.path()).await.unwrap();

    let config = CollectionConfig {
        name: "search_test".to_string(),
        dimension: 3,
        distance_metric: DistanceMetric::Cosine,
        vector_type: VectorType::Float32,
        index_config: IndexConfig::default(),
        quantization: None,
    };

    store.create_collection(&config).await.unwrap();

    // Insert test vectors
    let vectors = vec![
        Vector {
            id: Uuid::new_v4(),
            data: vec![1.0, 0.0, 0.0],
            metadata: None,
        },
        Vector {
            id: Uuid::new_v4(),
            data: vec![0.9, 0.1, 0.0],
            metadata: None,
        },
        Vector {
            id: Uuid::new_v4(),
            data: vec![0.0, 1.0, 0.0],
            metadata: None,
        },
    ];

    store.batch_insert("search_test", &vectors).await.unwrap();

    // Query
    let query_request = QueryRequest {
        collection: "search_test".to_string(),
        vector: vec![1.0, 0.0, 0.0],
        limit: 2,
        ef_search: None,
        filter: None,
    };

    let results = store.query(&query_request).await.unwrap();
    assert_eq!(results.len(), 2);

    // First result should be closest to query vector
    assert_eq!(results[0].id, vectors[0].id);
}

#[tokio::test]
async fn test_snapshot_management() {
    let temp_dir = tempfile::tempdir().unwrap();
    let store = VectorStore::new(temp_dir.path()).await.unwrap();

    let config = CollectionConfig {
        name: "snapshot_test".to_string(),
        dimension: 2,
        distance_metric: DistanceMetric::Euclidean,
        vector_type: VectorType::Float32,
        index_config: IndexConfig::default(),
        quantization: None,
    };

    store.create_collection(&config).await.unwrap();

    // Insert data
    let vector = Vector {
        id: Uuid::new_v4(),
        data: vec![1.0, 2.0],
        metadata: None,
    };
    store.insert("snapshot_test", &vector).await.unwrap();

    // Create snapshot
    let snapshot = store.create_snapshot("snapshot_test").await.unwrap();
    assert!(snapshot.name.starts_with("snapshot_test_"));
    assert!(snapshot.size_bytes > 0);

    // List snapshots
    let snapshots = store.list_snapshots("snapshot_test").await.unwrap();
    assert_eq!(snapshots.len(), 1);

    // Get snapshot
    let retrieved = store
        .get_snapshot("snapshot_test", &snapshot.name)
        .await
        .unwrap();
    assert_eq!(retrieved.name, snapshot.name);

    // Delete snapshot
    store
        .delete_snapshot("snapshot_test", &snapshot.name)
        .await
        .unwrap();

    let after_delete = store.list_snapshots("snapshot_test").await.unwrap();
    assert_eq!(after_delete.len(), 0);
}

#[tokio::test]
async fn test_recommend_api() {
    let temp_dir = tempfile::tempdir().unwrap();
    let store = VectorStore::new(temp_dir.path()).await.unwrap();

    let config = CollectionConfig {
        name: "recommend_test".to_string(),
        dimension: 3,
        distance_metric: DistanceMetric::Cosine,
        vector_type: VectorType::Float32,
        index_config: IndexConfig::default(),
        quantization: None,
    };

    store.create_collection(&config).await.unwrap();

    // Insert vectors
    let vec1_id = Uuid::new_v4();
    let vec2_id = Uuid::new_v4();
    let vec3_id = Uuid::new_v4();

    let vectors = vec![
        Vector {
            id: vec1_id,
            data: vec![1.0, 0.0, 0.0],
            metadata: None,
        },
        Vector {
            id: vec2_id,
            data: vec![0.9, 0.1, 0.0],
            metadata: None,
        },
        Vector {
            id: vec3_id,
            data: vec![0.0, 1.0, 0.0],
            metadata: None,
        },
    ];

    store
        .batch_insert("recommend_test", &vectors)
        .await
        .unwrap();

    // Recommend based on positive example
    let recommend_request = vectordb_common::search_api::RecommendRequest {
        collection: "recommend_test".to_string(),
        positive: vec![vec1_id],
        negative: vec![],
        filter: None,
        limit: 2,
        strategy: vectordb_common::search_api::RecommendStrategy::AverageVector,
        offset: 0,
    };

    let results = store.recommend(&recommend_request).await.unwrap();
    assert!(results.len() <= 2);
}
