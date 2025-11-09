use vectordb_common::sparse::*;

#[test]
fn test_sparse_vector_creation() {
    let sparse = SparseVector::new(vec![0, 2, 5], vec![1.0, 2.0, 3.0]);
    assert_eq!(sparse.nnz(), 3);
    assert_eq!(sparse.indices, vec![0, 2, 5]);
    assert_eq!(sparse.values, vec![1.0, 2.0, 3.0]);
}

#[test]
fn test_sparse_from_dense() {
    let dense = vec![0.1, 0.0, 0.5, 0.0, 0.9];
    let sparse = SparseVector::from_dense(&dense, 0.2);

    assert_eq!(sparse.nnz(), 2); // Only 0.5 and 0.9 above threshold
    assert_eq!(sparse.indices, vec![2, 4]);
    assert_eq!(sparse.values, vec![0.5, 0.9]);
}

#[test]
fn test_sparse_to_dense() {
    let sparse = SparseVector::new(vec![1, 3], vec![2.0, 4.0]);
    let dense = sparse.to_dense(5);

    assert_eq!(dense, vec![0.0, 2.0, 0.0, 4.0, 0.0]);
}

#[test]
fn test_sparse_dot_product() {
    let sparse1 = SparseVector::new(vec![0, 2, 4], vec![1.0, 2.0, 3.0]);
    let sparse2 = SparseVector::new(vec![1, 2, 4], vec![1.0, 2.0, 3.0]);

    // Common indices: 2 and 4
    // dot product: 2*2 + 3*3 = 4 + 9 = 13
    let dot = sparse1.dot(&sparse2);
    assert!((dot - 13.0).abs() < 1e-6);
}

#[test]
fn test_sparse_normalize() {
    let mut sparse = SparseVector::new(vec![0, 1], vec![3.0, 4.0]);
    sparse.normalize();

    // L2 norm of [3.0, 4.0] is 5.0
    // Normalized: [0.6, 0.8]
    assert!((sparse.values[0] - 0.6).abs() < 1e-6);
    assert!((sparse.values[1] - 0.8).abs() < 1e-6);
}

#[test]
fn test_sparse_cosine_similarity() {
    let sparse1 = SparseVector::new(vec![0, 1, 2], vec![1.0, 0.0, 0.0]);
    let sparse2 = SparseVector::new(vec![0, 1, 2], vec![1.0, 0.0, 0.0]);

    let similarity = sparse1.cosine_similarity(&sparse2);
    assert!((similarity - 1.0).abs() < 1e-6); // Identical vectors

    let sparse3 = SparseVector::new(vec![0, 1, 2], vec![0.0, 1.0, 0.0]);
    let similarity2 = sparse1.cosine_similarity(&sparse3);
    assert!((similarity2 - 0.0).abs() < 1e-6); // Orthogonal vectors
}

#[test]
fn test_multi_vector_creation() {
    let dense = vec![1.0, 2.0, 3.0];
    let sparse = SparseVector::new(vec![0, 5, 10], vec![1.0, 2.0, 3.0]);

    let multi = MultiVector::new(Some(dense.clone()), Some(sparse.clone()));

    assert!(multi.dense.is_some());
    assert!(multi.sparse.is_some());
    assert_eq!(multi.dense.unwrap(), dense);
}

#[test]
fn test_multi_vector_from_dense() {
    let dense = vec![1.0, 2.0, 3.0];
    let multi = MultiVector::from_dense(dense.clone());

    assert!(multi.dense.is_some());
    assert!(multi.sparse.is_none());
    assert_eq!(multi.dense.unwrap(), dense);
}

#[test]
fn test_multi_vector_from_sparse() {
    let sparse = SparseVector::new(vec![0, 1], vec![1.0, 2.0]);
    let multi = MultiVector::from_sparse(sparse.clone());

    assert!(multi.dense.is_none());
    assert!(multi.sparse.is_some());
    assert_eq!(multi.sparse.unwrap().indices, sparse.indices);
}

#[test]
fn test_bm25_default_params() {
    let bm25 = BM25::default();
    // Just verify it creates without panicking
    assert_eq!(bm25.doc_count, 0);
}

#[test]
fn test_bm25_add_document() {
    let mut bm25 = BM25::default();

    let doc1 = SparseVector::new(vec![1, 2, 3], vec![1.0, 1.0, 1.0]);
    let doc2 = SparseVector::new(vec![2, 3, 4], vec![1.0, 1.0, 1.0]);

    bm25.add_document(&doc1);
    assert_eq!(bm25.doc_count, 1);

    bm25.add_document(&doc2);
    assert_eq!(bm25.doc_count, 2);
}

#[test]
fn test_bm25_scoring() {
    let mut bm25 = BM25::default();

    // Add documents
    let doc1 = SparseVector::new(vec![0, 1, 2], vec![1.0, 1.0, 1.0]);
    let doc2 = SparseVector::new(vec![1, 2, 3], vec![1.0, 1.0, 1.0]);

    bm25.add_document(&doc1);
    bm25.add_document(&doc2);

    // Query
    let query = SparseVector::new(vec![1, 2], vec![1.0, 1.0]);

    // Score documents
    let score1 = bm25.score(&query, &doc1, doc1.nnz());
    let score2 = bm25.score(&query, &doc2, doc2.nnz());

    // Both docs contain query terms, scores should be positive
    assert!(score1 > 0.0);
    assert!(score2 > 0.0);
}

#[test]
fn test_hybrid_search_request_serialization() {
    let request = HybridSearchRequest {
        collection: "test".to_string(),
        dense: Some(vec![1.0, 2.0, 3.0]),
        sparse: Some(SparseVector::new(vec![0, 1], vec![1.0, 2.0])),
        fusion: FusionMethod::ReciprocalRankFusion,
        limit: 10,
        filter: None,
    };

    // Serialize and deserialize
    let json = serde_json::to_string(&request).unwrap();
    let deserialized: HybridSearchRequest = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.collection, "test");
    assert_eq!(deserialized.limit, 10);
    assert!(deserialized.dense.is_some());
    assert!(deserialized.sparse.is_some());
}

#[test]
fn test_fusion_method_serialization() {
    let methods = vec![
        FusionMethod::RelativeScoreFusion,
        FusionMethod::ReciprocalRankFusion,
        FusionMethod::DistributionBasedScoreFusion,
    ];

    for method in methods {
        let json = serde_json::to_string(&method).unwrap();
        let deserialized: FusionMethod = serde_json::from_str(&json).unwrap();
        // Just verify round-trip works
        let _ = deserialized;
    }
}
