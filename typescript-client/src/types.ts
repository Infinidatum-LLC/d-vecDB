/**
 * Distance metrics for vector similarity calculations
 */
export enum DistanceMetric {
  COSINE = 'Cosine',
  EUCLIDEAN = 'Euclidean',
  DOT_PRODUCT = 'DotProduct',
  MANHATTAN = 'Manhattan',
}

/**
 * Vector data types
 */
export enum VectorType {
  FLOAT32 = 'Float32',
  FLOAT16 = 'Float16',
  INT8 = 'Int8',
}

/**
 * HNSW index configuration
 */
export interface IndexConfig {
  /** Maximum number of connections per node in HNSW graph */
  maxConnections?: number;
  /** Size of dynamic candidate list during construction */
  efConstruction?: number;
  /** Size of dynamic candidate list during search */
  efSearch?: number;
  /** Maximum number of layers in HNSW graph */
  maxLayer?: number;
}

/**
 * Collection configuration
 */
export interface CollectionConfig {
  /** Collection name */
  name: string;
  /** Vector dimension */
  dimension: number;
  /** Distance metric for similarity calculation */
  distanceMetric: DistanceMetric;
  /** Vector data type */
  vectorType?: VectorType;
  /** Index configuration */
  indexConfig?: IndexConfig;
}

/**
 * Vector metadata (key-value pairs)
 */
export type VectorMetadata = Record<string, string | number | boolean>;

/**
 * Vector data (array of numbers or typed array)
 */
export type VectorData = number[] | Float32Array | Float64Array;

/**
 * Vector object
 */
export interface Vector {
  /** Vector ID */
  id: string;
  /** Vector data */
  data: VectorData;
  /** Optional metadata */
  metadata?: VectorMetadata;
}

/**
 * Query result
 */
export interface QueryResult {
  /** Vector ID */
  id: string;
  /** Distance/similarity score */
  distance: number;
  /** Optional metadata */
  metadata?: VectorMetadata;
}

/**
 * Search request
 */
export interface SearchRequest {
  /** Collection name */
  collectionName: string;
  /** Query vector */
  queryVector: VectorData;
  /** Maximum number of results */
  limit?: number;
  /** HNSW search parameter (overrides collection default) */
  efSearch?: number;
  /** Metadata filter */
  filter?: VectorMetadata;
}

/**
 * Search response
 */
export interface SearchResponse {
  /** Query results */
  results: QueryResult[];
}

/**
 * Collection information
 */
export interface CollectionInfo {
  /** Collection name */
  name: string;
  /** Vector dimension */
  dimension: number;
  /** Distance metric */
  distanceMetric: DistanceMetric;
  /** Vector type */
  vectorType: VectorType;
  /** Index configuration */
  indexConfig: IndexConfig;
}

/**
 * Collection statistics
 */
export interface CollectionStats {
  /** Collection name */
  name: string;
  /** Number of vectors */
  vectorCount: number;
  /** Vector dimension */
  dimension: number;
  /** Index size in bytes */
  indexSize: number;
  /** Memory usage in bytes */
  memoryUsage: number;
}

/**
 * Collection response
 */
export interface CollectionResponse {
  /** Collection information */
  collection: CollectionInfo;
  /** Success message */
  message?: string;
}

/**
 * List collections response
 */
export interface ListCollectionsResponse {
  /** List of collections */
  collections: CollectionInfo[];
}

/**
 * Insert response
 */
export interface InsertResponse {
  /** Success flag */
  success: boolean;
  /** Response message */
  message?: string;
  /** Number of vectors inserted */
  count?: number;
}

/**
 * Server statistics
 */
export interface ServerStats {
  /** Total number of vectors across all collections */
  totalVectors: number;
  /** Total number of collections */
  totalCollections: number;
  /** Memory usage in bytes */
  memoryUsage: number;
  /** Disk usage in bytes */
  diskUsage: number;
  /** Server uptime in seconds */
  uptimeSeconds: number;
}

/**
 * Health check response
 */
export interface HealthResponse {
  /** Health status */
  status: 'healthy' | 'unhealthy';
  /** Additional details */
  details?: Record<string, unknown>;
}

/**
 * Client configuration
 */
export interface ClientConfig {
  /** Server host */
  host?: string;
  /** Server port */
  port?: number;
  /** Request timeout in milliseconds */
  timeout?: number;
  /** Protocol (rest or grpc) */
  protocol?: 'rest' | 'grpc';
  /** Enable SSL/TLS */
  secure?: boolean;
  /** API key for authentication */
  apiKey?: string;
}
