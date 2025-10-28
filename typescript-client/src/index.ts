/**
 * d-vecDB TypeScript/JavaScript Client
 *
 * A high-performance vector database client for TypeScript and JavaScript.
 *
 * @packageDocumentation
 */

export { VectorDBClient } from './client';
export { RestClient } from './rest-client';

// Export types
export {
  DistanceMetric,
  VectorType,
  IndexConfig,
  CollectionConfig,
  VectorMetadata,
  VectorData,
  Vector,
  QueryResult,
  SearchRequest,
  SearchResponse,
  CollectionInfo,
  CollectionStats,
  CollectionResponse,
  ListCollectionsResponse,
  InsertResponse,
  ServerStats,
  HealthResponse,
  ClientConfig,
} from './types';

// Export exceptions
export {
  VectorDBError,
  ConnectionError,
  TimeoutError,
  AuthenticationError,
  AuthorizationError,
  CollectionNotFoundError,
  CollectionExistsError,
  VectorNotFoundError,
  InvalidParameterError,
  ValidationError,
  ServerError,
  RateLimitError,
  QuotaExceededError,
  ProtocolError,
} from './exceptions';
