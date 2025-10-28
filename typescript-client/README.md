# d-vecDB TypeScript/JavaScript Client

A high-performance TypeScript/JavaScript client for [d-vecDB](https://github.com/yourusername/d-vecDB), a blazingly fast vector database written in Rust.

[![npm version](https://badge.fury.io/js/d-vecdb.svg)](https://www.npmjs.com/package/d-vecdb)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Features

- 🚀 **High Performance**: Built on top of Rust-powered d-vecDB server
- 🎯 **Type-Safe**: Full TypeScript support with comprehensive type definitions
- 🔄 **Promise-Based**: Modern async/await API
- 📦 **Zero Dependencies**: Only axios for HTTP requests
- 🛡️ **Error Handling**: Comprehensive custom exceptions
- 📊 **HNSW Indexing**: Hierarchical Navigable Small World algorithm for fast similarity search
- 🎨 **Simple & Advanced APIs**: Both simple and full-featured methods
- 📝 **Well Documented**: Extensive examples and API documentation

## Installation

```bash
npm install d-vecdb
```

Or with yarn:

```bash
yarn add d-vecdb
```

## Quick Start

```typescript
import { VectorDBClient, DistanceMetric } from 'd-vecdb';

// Create client
const client = new VectorDBClient({
  host: 'localhost',
  port: 8080,
});

// Create a collection
await client.createCollectionSimple('my-collection', 128, DistanceMetric.COSINE);

// Insert vectors
await client.insertSimple('my-collection', 'vec-1', [0.1, 0.2, ...], { label: 'example' });

// Search for similar vectors
const results = await client.searchSimple('my-collection', [0.1, 0.2, ...], 10);
console.log(results);

// Clean up
client.close();
```

## Prerequisites

Before using this client, you need to have the d-vecDB server running:

```bash
# Install the server (if not already installed)
cargo install d-vecdb-server

# Start the server
d-vecdb-server --host 0.0.0.0 --port 8080
```

Or use Docker:

```bash
docker run -p 8080:8080 d-vecdb/server
```

## Usage Examples

### Creating a Collection

```typescript
import { VectorDBClient, DistanceMetric, VectorType } from 'd-vecdb';

const client = new VectorDBClient();

// Simple method
await client.createCollectionSimple('embeddings', 768, DistanceMetric.COSINE);

// Advanced method with custom configuration
await client.createCollection({
  name: 'embeddings',
  dimension: 768,
  distanceMetric: DistanceMetric.COSINE,
  vectorType: VectorType.FLOAT32,
  indexConfig: {
    maxConnections: 32,
    efConstruction: 400,
    efSearch: 100,
    maxLayer: 16,
  },
});
```

### Inserting Vectors

```typescript
// Insert a single vector
await client.insertSimple('embeddings', 'doc-1', vector, { title: 'Document 1' });

// Batch insert
const vectorsData: Array<[string, number[], Record<string, string>]> = [
  ['doc-1', vector1, { title: 'Document 1' }],
  ['doc-2', vector2, { title: 'Document 2' }],
  ['doc-3', vector3, { title: 'Document 3' }],
];

await client.batchInsertSimple('embeddings', vectorsData, 100);
```

### Searching

```typescript
// Simple search
const results = await client.searchSimple('embeddings', queryVector, 10);

results.forEach(result => {
  console.log(`ID: ${result.id}, Distance: ${result.distance}`);
  console.log(`Metadata: ${JSON.stringify(result.metadata)}`);
});

// Advanced search with filtering and custom HNSW parameters
const results = await client.searchSimple(
  'embeddings',
  queryVector,
  10,
  200, // efSearch - higher = more accurate, slower
  { category: 'technology' } // metadata filter
);
```

### Working with Vectors

```typescript
// Get a vector
const vector = await client.getVector('embeddings', 'doc-1');

// Update a vector
await client.updateVector('embeddings', {
  id: 'doc-1',
  data: newVector,
  metadata: { title: 'Updated Document' },
});

// Delete a vector
await client.deleteVector('embeddings', 'doc-1');
```

### Collection Management

```typescript
// List all collections
const collections = await client.listCollections();
console.log(collections.collections);

// Get collection info
const info = await client.getCollection('embeddings');
console.log(info.collection);

// Get collection statistics
const stats = await client.getCollectionStats('embeddings');
console.log(`Vectors: ${stats.vectorCount}, Memory: ${stats.memoryUsage} bytes`);

// Delete a collection
await client.deleteCollection('embeddings');
```

### Server Operations

```typescript
// Check server health
const isAlive = await client.ping();
console.log(`Server is ${isAlive ? 'reachable' : 'unreachable'}`);

// Get server statistics
const stats = await client.getServerStats();
console.log(`Total vectors: ${stats.totalVectors}`);
console.log(`Total collections: ${stats.totalCollections}`);
console.log(`Uptime: ${stats.uptimeSeconds} seconds`);
```

## API Reference

### VectorDBClient

#### Constructor

```typescript
new VectorDBClient(config?: ClientConfig)
```

**ClientConfig:**
- `host?: string` - Server host (default: 'localhost')
- `port?: number` - Server port (default: 8080)
- `timeout?: number` - Request timeout in ms (default: 30000)
- `protocol?: 'rest' | 'grpc'` - Protocol to use (default: 'rest')
- `secure?: boolean` - Use HTTPS (default: false)
- `apiKey?: string` - API key for authentication

#### Collection Methods

- `createCollection(config: CollectionConfig): Promise<CollectionResponse>`
- `createCollectionSimple(name: string, dimension: number, distanceMetric?: DistanceMetric): Promise<CollectionResponse>`
- `listCollections(): Promise<ListCollectionsResponse>`
- `getCollection(name: string): Promise<CollectionResponse>`
- `getCollectionStats(name: string): Promise<CollectionStats>`
- `deleteCollection(name: string): Promise<CollectionResponse>`

#### Vector Methods

- `insertVector(collectionName: string, vector: Vector): Promise<InsertResponse>`
- `insertSimple(collectionName: string, vectorId: string, vectorData: VectorData, metadata?: VectorMetadata): Promise<InsertResponse>`
- `insertVectors(collectionName: string, vectors: Vector[]): Promise<InsertResponse>`
- `batchInsertSimple(collectionName: string, vectorsData: Array<[string, VectorData, VectorMetadata?]>, batchSize?: number): Promise<InsertResponse[]>`
- `getVector(collectionName: string, vectorId: string): Promise<Vector>`
- `updateVector(collectionName: string, vector: Vector): Promise<InsertResponse>`
- `deleteVector(collectionName: string, vectorId: string): Promise<InsertResponse>`

#### Search Methods

- `search(request: SearchRequest): Promise<SearchResponse>`
- `searchSimple(collectionName: string, queryVector: VectorData, limit?: number, efSearch?: number, filter?: VectorMetadata): Promise<QueryResult[]>`

#### Server Methods

- `getServerStats(): Promise<ServerStats>`
- `healthCheck(): Promise<HealthResponse>`
- `ping(): Promise<boolean>`
- `getInfo(): Record<string, unknown>`
- `close(): void`

### Types

#### DistanceMetric

```typescript
enum DistanceMetric {
  COSINE = 'Cosine',
  EUCLIDEAN = 'Euclidean',
  DOT_PRODUCT = 'DotProduct',
  MANHATTAN = 'Manhattan',
}
```

#### VectorType

```typescript
enum VectorType {
  FLOAT32 = 'Float32',
  FLOAT16 = 'Float16',
  INT8 = 'Int8',
}
```

### Error Handling

The client provides comprehensive custom exceptions:

```typescript
import {
  VectorDBError,
  ConnectionError,
  TimeoutError,
  CollectionNotFoundError,
  VectorNotFoundError,
  InvalidParameterError,
} from 'd-vecdb';

try {
  await client.getCollection('non-existent');
} catch (error) {
  if (error instanceof CollectionNotFoundError) {
    console.error('Collection not found:', error.message);
  } else if (error instanceof ConnectionError) {
    console.error('Cannot connect to server:', error.message);
  } else {
    console.error('Unexpected error:', error);
  }
}
```

Available exceptions:
- `VectorDBError` - Base exception
- `ConnectionError` - Connection issues
- `TimeoutError` - Request timeout
- `AuthenticationError` - Authentication failure
- `AuthorizationError` - Permission denied
- `CollectionNotFoundError` - Collection doesn't exist
- `CollectionExistsError` - Collection already exists
- `VectorNotFoundError` - Vector not found
- `InvalidParameterError` - Invalid parameters
- `ValidationError` - Validation failure
- `ServerError` - Server-side error
- `RateLimitError` - Rate limit exceeded
- `QuotaExceededError` - Quota exceeded
- `ProtocolError` - Protocol error

## Advanced Usage

### Using TypedArrays

```typescript
// Use Float32Array for better performance
const vector = new Float32Array(768);
for (let i = 0; i < 768; i++) {
  vector[i] = Math.random();
}

await client.insertSimple('embeddings', 'vec-1', vector);
```

### Parallel Operations

```typescript
// Parallel searches
const queries = [vector1, vector2, vector3, vector4];
const results = await Promise.all(
  queries.map(query => client.searchSimple('embeddings', query, 10))
);

// Parallel batch inserts
const batches = chunkArray(allVectors, 1000);
await Promise.all(
  batches.map(batch => client.batchInsertSimple('embeddings', batch))
);
```

### Custom HNSW Parameters

HNSW (Hierarchical Navigable Small World) parameters affect search performance:

- **maxConnections**: Number of bi-directional links per node (default: 16)
  - Higher = better recall, more memory
- **efConstruction**: Size of dynamic candidate list during construction (default: 200)
  - Higher = better quality index, slower build
- **efSearch**: Size of dynamic candidate list during search (default: 50)
  - Higher = better recall, slower search
- **maxLayer**: Maximum number of layers (default: 16)

```typescript
await client.createCollection({
  name: 'embeddings',
  dimension: 768,
  distanceMetric: DistanceMetric.COSINE,
  indexConfig: {
    maxConnections: 32, // Increase for better recall
    efConstruction: 400, // Increase for better quality
    efSearch: 100, // Override per search if needed
  },
});
```

## Performance Tips

1. **Batch Operations**: Use `batchInsertSimple` for bulk inserts
2. **Parallel Requests**: Leverage Promise.all for concurrent operations
3. **Typed Arrays**: Use Float32Array for better memory efficiency
4. **Connection Reuse**: Reuse the same client instance
5. **HNSW Tuning**: Adjust efSearch based on accuracy/speed tradeoff

## Examples

Check out the [examples directory](./src/examples) for more:

- [basic-usage.ts](./src/examples/basic-usage.ts) - Complete basic usage example
- [advanced-search.ts](./src/examples/advanced-search.ts) - Advanced search with filtering and HNSW tuning

## Building from Source

```bash
# Clone the repository
git clone https://github.com/yourusername/d-vecDB.git
cd d-vecDB/typescript-client

# Install dependencies
npm install

# Build
npm run build

# Run tests
npm test

# Run linter
npm run lint
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Links

- [d-vecDB Main Repository](https://github.com/yourusername/d-vecDB)
- [npm Package](https://www.npmjs.com/package/d-vecdb)
- [Documentation](https://github.com/yourusername/d-vecDB/tree/main/typescript-client)
- [Issue Tracker](https://github.com/yourusername/d-vecDB/issues)

## Support

If you encounter any issues or have questions:

1. Check the [documentation](https://github.com/yourusername/d-vecDB)
2. Search [existing issues](https://github.com/yourusername/d-vecDB/issues)
3. Open a [new issue](https://github.com/yourusername/d-vecDB/issues/new)

## Acknowledgments

- Built on top of [d-vecDB](https://github.com/yourusername/d-vecDB) - A high-performance vector database written in Rust
- Uses [HNSW](https://arxiv.org/abs/1603.09320) algorithm for efficient approximate nearest neighbor search
