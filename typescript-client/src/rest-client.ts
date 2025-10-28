import axios, { AxiosInstance, AxiosError } from 'axios';
import {
  ClientConfig,
  CollectionConfig,
  CollectionResponse,
  ListCollectionsResponse,
  CollectionStats,
  CollectionInfo,
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
import {
  VectorDBError,
  ConnectionError,
  TimeoutError,
  AuthenticationError,
  CollectionNotFoundError,
  CollectionExistsError,
  VectorNotFoundError,
  InvalidParameterError,
  ServerError,
} from './exceptions';

/**
 * REST client for d-vecDB
 */
export class RestClient {
  private client: AxiosInstance;
  private baseUrl: string;

  constructor(config: ClientConfig = {}) {
    const {
      host = 'localhost',
      port = 8080,
      timeout = 30000,
      secure = false,
      apiKey,
    } = config;

    const protocol = secure ? 'https' : 'http';
    this.baseUrl = `${protocol}://${host}:${port}`;

    this.client = axios.create({
      baseURL: this.baseUrl,
      timeout,
      headers: {
        'Content-Type': 'application/json',
        ...(apiKey && { Authorization: `Bearer ${apiKey}` }),
      },
    });

    // Add response interceptor for error handling
    this.client.interceptors.response.use(
      response => response,
      error => {
        throw this.handleError(error);
      }
    );
  }

  /**
   * Unwrap the server's standard response format
   * Server returns: {"success": true, "data": <actual_data>, "error": null}
   */
  private unwrapResponse<T>(responseData: {
    success?: boolean;
    data?: T;
    error?: string | null;
  }): T {
    // If the response has the standard wrapper format, extract the data
    if ('success' in responseData && 'data' in responseData) {
      if (responseData.error) {
        throw new ServerError(responseData.error);
      }
      return responseData.data as T;
    }
    // Otherwise, return the data as-is (for backward compatibility)
    return responseData as unknown as T;
  }

  /**
   * Handle axios errors and convert to custom exceptions
   */
  private handleError(error: AxiosError): VectorDBError {
    if (error.code === 'ECONNREFUSED' || error.code === 'ENOTFOUND') {
      return new ConnectionError(`Failed to connect to server at ${this.baseUrl}`);
    }

    if (error.code === 'ECONNABORTED' || error.message.includes('timeout')) {
      return new TimeoutError('Request timed out');
    }

    if (error.response) {
      const status = error.response.status;
      const message = (error.response.data as { message?: string })?.message || error.message;

      switch (status) {
        case 401:
          return new AuthenticationError(message);
        case 404:
          if (message.toLowerCase().includes('collection')) {
            const match = message.match(/Collection '([^']+)'/);
            const collectionName = match ? match[1] : 'unknown';
            return new CollectionNotFoundError(collectionName);
          }
          if (message.toLowerCase().includes('vector')) {
            const match = message.match(/Vector '([^']+)'/);
            const vectorId = match ? match[1] : 'unknown';
            return new VectorNotFoundError(vectorId);
          }
          return new ServerError(message, status);
        case 409:
          if (message.toLowerCase().includes('already exists')) {
            const match = message.match(/Collection '([^']+)'/);
            const collectionName = match ? match[1] : 'unknown';
            return new CollectionExistsError(collectionName);
          }
          return new ServerError(message, status);
        case 400:
          return new InvalidParameterError(message);
        case 500:
        case 502:
        case 503:
        case 504:
          return new ServerError(message, status);
        default:
          return new ServerError(message, status);
      }
    }

    return new VectorDBError(error.message);
  }

  /**
   * Create a new collection
   */
  async createCollection(config: CollectionConfig): Promise<CollectionResponse> {
    const response = await this.client.post('/collections', {
      name: config.name,
      dimension: config.dimension,
      distance_metric: config.distanceMetric,
      vector_type: config.vectorType,
      index_config: config.indexConfig
        ? {
            max_connections: config.indexConfig.maxConnections,
            ef_construction: config.indexConfig.efConstruction,
            ef_search: config.indexConfig.efSearch,
            max_layer: config.indexConfig.maxLayer,
          }
        : undefined,
    });
    const data = this.unwrapResponse<unknown>(response.data);
    return this.transformCollectionResponse(data);
  }

  /**
   * List all collections
   */
  async listCollections(): Promise<ListCollectionsResponse> {
    const response = await this.client.get('/collections');
    const data = this.unwrapResponse<unknown[]>(response.data);
    return {
      collections: (Array.isArray(data) ? data : []).map((c: unknown) =>
        this.transformCollectionInfo(c)
      ),
    };
  }

  /**
   * Get collection information
   */
  async getCollection(name: string): Promise<CollectionResponse> {
    const response = await this.client.get(`/collections/${name}`);
    const data = this.unwrapResponse<unknown>(response.data);
    return this.transformCollectionResponse(data);
  }

  /**
   * Get collection statistics
   */
  async getCollectionStats(name: string): Promise<CollectionStats> {
    const response = await this.client.get(`/collections/${name}/stats`);
    const data = this.unwrapResponse<{
      name: string;
      vector_count: number;
      dimension: number;
      index_size: number;
      memory_usage: number;
    }>(response.data);
    return {
      name: data.name,
      vectorCount: data.vector_count,
      dimension: data.dimension,
      indexSize: data.index_size,
      memoryUsage: data.memory_usage,
    };
  }

  /**
   * Delete a collection
   */
  async deleteCollection(name: string): Promise<CollectionResponse> {
    const response = await this.client.delete(`/collections/${name}`);
    const data = this.unwrapResponse<unknown>(response.data);
    return this.transformCollectionResponse(data);
  }

  /**
   * Insert a single vector
   */
  async insertVector(collectionName: string, vector: Vector): Promise<InsertResponse> {
    const response = await this.client.post(`/collections/${collectionName}/vectors`, {
      id: vector.id,
      data: Array.from(vector.data),
      metadata: vector.metadata,
    });
    const data = this.unwrapResponse<{ message?: string }>(response.data);
    return {
      success: true,
      message: data.message,
      count: 1,
    };
  }

  /**
   * Batch insert vectors
   */
  async batchInsert(collectionName: string, vectors: Vector[]): Promise<InsertResponse> {
    const response = await this.client.post(`/collections/${collectionName}/vectors/batch`, {
      vectors: vectors.map(v => ({
        id: v.id,
        data: Array.from(v.data),
        metadata: v.metadata,
      })),
    });
    const data = this.unwrapResponse<{ message?: string }>(response.data);
    return {
      success: true,
      message: data.message,
      count: vectors.length,
    };
  }

  /**
   * Get a vector by ID
   */
  async getVector(collectionName: string, vectorId: string): Promise<Vector> {
    const response = await this.client.get(`/collections/${collectionName}/vectors/${vectorId}`);
    const data = this.unwrapResponse<{ id: string; data: number[]; metadata?: VectorMetadata }>(
      response.data
    );
    return {
      id: data.id,
      data: data.data,
      metadata: data.metadata,
    };
  }

  /**
   * Update a vector
   */
  async updateVector(collectionName: string, vector: Vector): Promise<InsertResponse> {
    const response = await this.client.put(`/collections/${collectionName}/vectors/${vector.id}`, {
      data: Array.from(vector.data),
      metadata: vector.metadata,
    });
    const data = this.unwrapResponse<{ message?: string }>(response.data);
    return {
      success: true,
      message: data.message,
    };
  }

  /**
   * Delete a vector
   */
  async deleteVector(collectionName: string, vectorId: string): Promise<InsertResponse> {
    const response = await this.client.delete(
      `/collections/${collectionName}/vectors/${vectorId}`
    );
    const data = this.unwrapResponse<{ message?: string }>(response.data);
    return {
      success: true,
      message: data.message,
    };
  }

  /**
   * Search for similar vectors
   */
  async search(request: SearchRequest): Promise<SearchResponse> {
    const response = await this.client.post(`/collections/${request.collectionName}/search`, {
      query_vector: Array.from(request.queryVector),
      limit: request.limit || 10,
      ef_search: request.efSearch,
      filter: request.filter,
    });
    const data = this.unwrapResponse<
      { id: string; distance: number; metadata?: VectorMetadata }[]
    >(response.data);
    return {
      results: (Array.isArray(data) ? data : []).map(r => ({
        id: r.id,
        distance: r.distance,
        metadata: r.metadata,
      })),
    };
  }

  /**
   * Get server statistics
   */
  async getStats(): Promise<ServerStats> {
    const response = await this.client.get('/stats');
    const data = this.unwrapResponse<{
      total_vectors: number;
      total_collections: number;
      memory_usage: number;
      disk_usage: number;
      uptime_seconds: number;
    }>(response.data);
    return {
      totalVectors: data.total_vectors,
      totalCollections: data.total_collections,
      memoryUsage: data.memory_usage,
      diskUsage: data.disk_usage,
      uptimeSeconds: data.uptime_seconds,
    };
  }

  /**
   * Health check
   */
  async health(): Promise<HealthResponse> {
    const response = await this.client.get('/health');
    const data = this.unwrapResponse<{
      status: 'healthy' | 'unhealthy';
      details?: Record<string, unknown>;
    }>(response.data);
    return {
      status: data.status,
      details: data.details,
    };
  }

  /**
   * Ping the server
   */
  async ping(): Promise<boolean> {
    try {
      await this.health();
      return true;
    } catch {
      return false;
    }
  }

  /**
   * Transform collection response from API format to client format
   */
  private transformCollectionResponse(data: unknown): CollectionResponse {
    const d = data as {
      collection?: unknown;
      message?: string;
      name?: string;
      dimension?: number;
      distance_metric?: string;
      vector_type?: string;
      index_config?: unknown;
    };
    return {
      collection: this.transformCollectionInfo(d.collection || d),
      message: d.message,
    };
  }

  /**
   * Transform collection info from API format to client format
   */
  private transformCollectionInfo(data: unknown): CollectionInfo {
    const d = data as {
      name: string;
      dimension: number;
      distance_metric: string;
      vector_type: string;
      index_config?: {
        max_connections?: number;
        ef_construction?: number;
        ef_search?: number;
        max_layer?: number;
      };
    };
    return {
      name: d.name,
      dimension: d.dimension,
      distanceMetric: d.distance_metric as DistanceMetric,
      vectorType: d.vector_type as VectorType,
      indexConfig: {
        maxConnections: d.index_config?.max_connections,
        efConstruction: d.index_config?.ef_construction,
        efSearch: d.index_config?.ef_search,
        maxLayer: d.index_config?.max_layer,
      },
    };
  }
}
