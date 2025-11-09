use vectordb_common::search_api::*;
use uuid::Uuid;

#[test]
fn test_recommend_request_default() {
    let request = RecommendRequest {
        collection: "test".to_string(),
        positive: vec![Uuid::new_v4()],
        negative: vec![],
        filter: None,
        limit: 10,
        strategy: RecommendStrategy::default(),
        offset: 0,
    };

    assert_eq!(request.limit, 10);
    assert!(matches!(
        request.strategy,
        RecommendStrategy::AverageVector
    ));
}

#[test]
fn test_recommend_strategy_serialization() {
    let strategy = RecommendStrategy::AverageVector;
    let json = serde_json::to_string(&strategy).unwrap();
    assert_eq!(json, "\"average_vector\"");

    let deserialized: RecommendStrategy = serde_json::from_str(&json).unwrap();
    assert!(matches!(deserialized, RecommendStrategy::AverageVector));
}

#[test]
fn test_discovery_request_creation() {
    let request = DiscoveryRequest {
        collection: "test".to_string(),
        target: DiscoveryTarget::Vector(vec![1.0, 2.0, 3.0]),
        context: vec![ContextPair {
            positive: Uuid::new_v4(),
            negative: Uuid::new_v4(),
        }],
        filter: None,
        limit: 10,
        offset: 0,
    };

    assert_eq!(request.context.len(), 1);
    assert_eq!(request.limit, 10);
}

#[test]
fn test_discovery_target_vector_id() {
    let id = Uuid::new_v4();
    let target = DiscoveryTarget::VectorId(id);

    if let DiscoveryTarget::VectorId(extracted_id) = target {
        assert_eq!(extracted_id, id);
    } else {
        panic!("Expected VectorId variant");
    }
}

#[test]
fn test_discovery_target_vector() {
    let vec = vec![1.0, 2.0, 3.0];
    let target = DiscoveryTarget::Vector(vec.clone());

    if let DiscoveryTarget::Vector(extracted_vec) = target {
        assert_eq!(extracted_vec, vec);
    } else {
        panic!("Expected Vector variant");
    }
}

#[test]
fn test_scroll_request_defaults() {
    let request = ScrollRequest {
        collection: "test".to_string(),
        filter: None,
        limit: 100,
        offset: None,
        with_vectors: true,
        with_payload: true,
    };

    assert!(request.with_vectors);
    assert!(request.with_payload);
    assert!(request.offset.is_none());
}

#[test]
fn test_scroll_response_creation() {
    let response = ScrollResponse {
        points: vec![ScoredPoint {
            id: Uuid::new_v4(),
            score: 0.95,
            vector: Some(vec![1.0, 2.0, 3.0]),
            payload: None,
        }],
        next_offset: Some("cursor_123".to_string()),
    };

    assert_eq!(response.points.len(), 1);
    assert!(response.next_offset.is_some());
}

#[test]
fn test_count_request_exact_flag() {
    let request = CountRequest {
        collection: "test".to_string(),
        filter: None,
        exact: true,
    };

    assert!(request.exact);
}

#[test]
fn test_count_response() {
    let response = CountResponse { count: 12345 };
    assert_eq!(response.count, 12345);
}

#[test]
fn test_batch_search_request() {
    let request = BatchSearchRequest {
        collection: "test".to_string(),
        searches: vec![
            SearchQuery {
                vector: vec![1.0, 2.0, 3.0],
                filter: None,
                limit: 10,
                offset: 0,
            },
            SearchQuery {
                vector: vec![4.0, 5.0, 6.0],
                filter: None,
                limit: 5,
                offset: 0,
            },
        ],
    };

    assert_eq!(request.searches.len(), 2);
    assert_eq!(request.searches[0].limit, 10);
    assert_eq!(request.searches[1].limit, 5);
}

#[test]
fn test_average_vectors() {
    let vectors = vec![vec![1.0, 2.0, 3.0], vec![3.0, 4.0, 5.0]];

    let avg = average_vectors(&vectors).unwrap();
    assert_eq!(avg, vec![2.0, 3.0, 4.0]);
}

#[test]
fn test_average_vectors_empty() {
    let vectors: Vec<Vec<f32>> = vec![];
    let avg = average_vectors(&vectors);
    assert!(avg.is_none());
}

#[test]
fn test_average_vectors_dimension_mismatch() {
    let vectors = vec![vec![1.0, 2.0], vec![3.0, 4.0, 5.0]];
    let avg = average_vectors(&vectors);
    assert!(avg.is_none());
}

#[test]
fn test_compute_recommendation_vector() {
    let positive = vec![vec![1.0, 0.0], vec![0.9, 0.1]];
    let negative = vec![vec![-1.0, 0.0]];

    let rec = compute_recommendation_vector(&positive, &negative).unwrap();

    // Expected: 2 * avg(positive) - avg(negative)
    // avg(positive) = [0.95, 0.05]
    // avg(negative) = [-1.0, 0.0]
    // result = 2 * [0.95, 0.05] - [-1.0, 0.0] = [2.9, 0.1]
    assert!((rec[0] - 2.9).abs() < 0.01);
    assert!((rec[1] - 0.1).abs() < 0.01);
}

#[test]
fn test_compute_recommendation_vector_no_negative() {
    let positive = vec![vec![1.0, 2.0]];
    let negative = vec![];

    let rec = compute_recommendation_vector(&positive, &negative).unwrap();
    assert_eq!(rec, vec![1.0, 2.0]); // Should return average of positive
}

#[test]
fn test_compute_discovery_direction() {
    let target = vec![0.0, 0.0];
    let context = vec![
        (vec![1.0, 0.0], vec![-1.0, 0.0]), // Direction: [2, 0]
        (vec![0.0, 1.0], vec![0.0, -1.0]), // Direction: [0, 2]
    ];

    let result = compute_discovery_direction(&target, &context).unwrap();

    // Average direction: ([2, 0] + [0, 2]) / 2 = [1, 1]
    // Applied to target [0, 0]: result = [1, 1]
    assert_eq!(result, vec![1.0, 1.0]);
}

#[test]
fn test_compute_discovery_direction_no_context() {
    let target = vec![1.0, 2.0, 3.0];
    let context = vec![];

    let result = compute_discovery_direction(&target, &context).unwrap();
    assert_eq!(result, target); // Should return target unchanged
}

#[test]
fn test_compute_discovery_direction_dimension_mismatch() {
    let target = vec![1.0, 2.0];
    let context = vec![(vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0])];

    let result = compute_discovery_direction(&target, &context);
    assert!(result.is_none()); // Dimension mismatch
}

#[test]
fn test_scored_point_serialization() {
    let point = ScoredPoint {
        id: Uuid::new_v4(),
        score: 0.95,
        vector: Some(vec![1.0, 2.0, 3.0]),
        payload: Some(
            vec![("key".to_string(), serde_json::json!("value"))]
                .into_iter()
                .collect(),
        ),
    };

    let json = serde_json::to_string(&point).unwrap();
    let deserialized: ScoredPoint = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.score, 0.95);
    assert!(deserialized.vector.is_some());
    assert!(deserialized.payload.is_some());
}

#[test]
fn test_search_query_creation() {
    let query = SearchQuery {
        vector: vec![1.0, 2.0, 3.0],
        filter: None,
        limit: 20,
        offset: 5,
    };

    assert_eq!(query.limit, 20);
    assert_eq!(query.offset, 5);
    assert_eq!(query.vector.len(), 3);
}
