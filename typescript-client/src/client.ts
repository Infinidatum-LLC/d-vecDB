import { RestClient } from './rest-client';
import {
  ClientConfig,
  CollectionConfig,
  CollectionResponse,
  ListCollectionsResponse,
  CollectionStats,
  Vector,
  InsertResponse,
  SearchRequest,
  SearchResponse,
  QueryResult,
  ServerStats,
  HealthResponse,
  VectorData,
  VectorMetadata,
  DistanceMetric,
  VectorType,
} from './types';

/**
 * Main d-vecDB client
 *
 * @example
 * ```typescript
 * const client = new VectorDBClient({ host: 'localhost', port: 8080 });
 *
 * // Create a collection
 * await client.createCollectionSimple('my-collection', 128, DistanceMetric.COSINE);
 *
 * // Insert vectors
 * await client.insertSimple('my-collection', 'vec-1', [0.1, 0.2, ...], { label: 'example' });
 *
 * // Search
 * const results = await client.searchSimple('my-collection', [0.1, 0.2, ...], 10);
 *
 * client.close();
 * ```
 */
export class VectorDBClient {
  private restClient: RestClient;

  /**
   * Create a new VectorDB client
   *
   * @param config - Client configuration
   */
  constructor(config: ClientConfig = {}) {
    this.restClient = new RestClient(config);
  }

  // ==================== Collection Management ====================

  /**
   * Create a new collection with full configuration
   *
   * @param config - Collection configuration
   * @returns Collection response
   */
  async createCollection(config: CollectionConfig): Promise<CollectionResponse> {
    return this.restClient.createCollection(config);
  }

  /**
   * Create a new collection with simple parameters
   *
   * @param name - Collection name
   * @param dimension - Vector dimension
   * @param distanceMetric - Distance metric (default: COSINE)
   * @param vectorType - Vector data type (default: FLOAT32)
   * @returns Collection response
   */
  async createCollectionSimple(
    name: string,
    dimension: number,
    distanceMetric: DistanceMetric = DistanceMetric.COSINE,
    vectorType: VectorType = VectorType.FLOAT32
  ): Promise<CollectionResponse> {
    return this.restClient.createCollection({
      name,
      dimension,
      distanceMetric,
      vectorType,
    });
  }

  /**
   * List all collection names (fast - returns only names)
   *
   * @returns Array of collection names
   */
  async listCollectionNames(): Promise<string[]> {
    return this.restClient.listCollectionNames();
  }

  /**
   * List all collections with full details (slower - fetches each collection)
   *
   * @returns List of collections with complete information
   */
  async listCollections(): Promise<ListCollectionsResponse> {
    return this.restClient.listCollections();
  }

  /**
   * Get collection information
   *
   * @param name - Collection name
   * @returns Collection response
   */
  async getCollection(name: string): Promise<CollectionResponse> {
    return this.restClient.getCollection(name);
  }

  /**
   * Get collection statistics
   *
   * @param name - Collection name
   * @returns Collection statistics
   */
  async getCollectionStats(name: string): Promise<CollectionStats> {
    return this.restClient.getCollectionStats(name);
  }

  /**
   * Delete a collection
   *
   * @param name - Collection name
   * @returns Collection response
   */
  async deleteCollection(name: string): Promise<CollectionResponse> {
    return this.restClient.deleteCollection(name);
  }

  // ==================== Vector Operations ====================

  /**
   * Insert a single vector
   *
   * @param collectionName - Collection name
   * @param vector - Vector to insert
   * @returns Insert response
   */
  async insertVector(collectionName: string, vector: Vector): Promise<InsertResponse> {
    return this.restClient.insertVector(collectionName, vector);
  }

  /**
   * Insert a single vector with simple parameters
   *
   * @param collectionName - Collection name
   * @param vectorId - Vector ID
   * @param vectorData - Vector data
   * @param metadata - Optional metadata
   * @returns Insert response
   */
  async insertSimple(
    collectionName: string,
    vectorId: string,
    vectorData: VectorData,
    metadata?: VectorMetadata
  ): Promise<InsertResponse> {
    return this.restClient.insertVector(collectionName, {
      id: vectorId,
      data: vectorData,
      metadata,
    });
  }

  /**
   * Batch insert multiple vectors
   *
   * @param collectionName - Collection name
   * @param vectors - Array of vectors
   * @returns Insert response
   */
  async insertVectors(collectionName: string, vectors: Vector[]): Promise<InsertResponse> {
    return this.restClient.batchInsert(collectionName, vectors);
  }

  /**
   * Batch insert vectors with simple parameters
   *
   * @param collectionName - Collection name
   * @param vectorsData - Array of [id, data, metadata?] tuples
   * @param batchSize - Batch size for chunking (default: 100)
   * @returns Array of insert responses
   */
  async batchInsertSimple(
    collectionName: string,
    vectorsData: Array<[string, VectorData, VectorMetadata?]>,
    batchSize = 100
  ): Promise<InsertResponse[]> {
    const results: InsertResponse[] = [];

    // Process in batches
    for (let i = 0; i < vectorsData.length; i += batchSize) {
      const batch = vectorsData.slice(i, i + batchSize);
      const vectors: Vector[] = batch.map(([id, data, metadata]) => ({
        id,
        data,
        metadata,
      }));

      const response = await this.restClient.batchInsert(collectionName, vectors);
      results.push(response);
    }

    return results;
  }

  /**
   * Get a vector by ID
   *
   * @param collectionName - Collection name
   * @param vectorId - Vector ID
   * @returns Vector
   */
  async getVector(collectionName: string, vectorId: string): Promise<Vector> {
    return this.restClient.getVector(collectionName, vectorId);
  }

  /**
   * Update a vector
   *
   * @param collectionName - Collection name
   * @param vector - Updated vector
   * @returns Insert response
   */
  async updateVector(collectionName: string, vector: Vector): Promise<InsertResponse> {
    return this.restClient.updateVector(collectionName, vector);
  }

  /**
   * Delete a vector
   *
   * @param collectionName - Collection name
   * @param vectorId - Vector ID
   * @returns Insert response
   */
  async deleteVector(collectionName: string, vectorId: string): Promise<InsertResponse> {
    return this.restClient.deleteVector(collectionName, vectorId);
  }

  // ==================== Search Operations ====================

  /**
   * Search for similar vectors
   *
   * @param request - Search request
   * @returns Search response
   */
  async search(request: SearchRequest): Promise<SearchResponse> {
    return this.restClient.search(request);
  }

  /**
   * Search for similar vectors with simple parameters
   *
   * @param collectionName - Collection name
   * @param queryVector - Query vector
   * @param limit - Maximum number of results (default: 10)
   * @param efSearch - HNSW search parameter (optional)
   * @param filter - Metadata filter (optional)
   * @returns Array of query results
   */
  async searchSimple(
    collectionName: string,
    queryVector: VectorData,
    limit = 10,
    efSearch?: number,
    filter?: VectorMetadata
  ): Promise<QueryResult[]> {
    const response = await this.restClient.search({
      collectionName,
      queryVector,
      limit,
      efSearch,
      filter,
    });
    return response.results;
  }

  // ==================== Server Operations ====================

  /**
   * Get server statistics
   *
   * @returns Server statistics
   */
  async getServerStats(): Promise<ServerStats> {
    return this.restClient.getStats();
  }

  /**
   * Health check
   *
   * @returns Health response
   */
  async healthCheck(): Promise<HealthResponse> {
    return this.restClient.health();
  }

  /**
   * Ping the server
   *
   * @returns true if server is reachable
   */
  async ping(): Promise<boolean> {
    return this.restClient.ping();
  }

  /**
   * Get client information
   *
   * @returns Client information
   */
  getInfo(): Record<string, unknown> {
    return {
      version: '0.1.0',
      protocol: 'rest',
      client: 'd-vecdb-typescript',
    };
  }

  /**
   * Close the client (cleanup resources)
   */
  close(): void {
    // Currently no cleanup needed, but method provided for consistency with other clients
  }
}
