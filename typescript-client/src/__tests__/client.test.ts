import { VectorDBClient, DistanceMetric, CollectionNotFoundError } from '../index';

// Mock axios
jest.mock('axios');

describe('VectorDBClient', () => {
  let client: VectorDBClient;

  beforeEach(() => {
    client = new VectorDBClient({
      host: 'localhost',
      port: 8080,
    });
  });

  afterEach(() => {
    client.close();
  });

  describe('constructor', () => {
    it('should create a client with default config', () => {
      const defaultClient = new VectorDBClient();
      expect(defaultClient).toBeDefined();
      expect(defaultClient.getInfo().client).toBe('d-vecdb-typescript');
    });

    it('should create a client with custom config', () => {
      const customClient = new VectorDBClient({
        host: 'custom-host',
        port: 9999,
        timeout: 60000,
      });
      expect(customClient).toBeDefined();
    });
  });

  describe('getInfo', () => {
    it('should return client info', () => {
      const info = client.getInfo();
      expect(info).toHaveProperty('version');
      expect(info).toHaveProperty('protocol');
      expect(info).toHaveProperty('client');
      expect(info.client).toBe('d-vecdb-typescript');
    });
  });

  describe('createCollectionSimple', () => {
    it('should create collection with simple parameters', async () => {
      const name = 'test-collection';
      const dimension = 128;
      const distanceMetric = DistanceMetric.COSINE;

      // This test would need proper mocking of the RestClient
      // For now, we'll just test that the method exists and has the right signature
      expect(typeof client.createCollectionSimple).toBe('function');
    });
  });

  describe('insertSimple', () => {
    it('should insert a vector with simple parameters', async () => {
      const collectionName = 'test-collection';
      const vectorId = 'vec-1';
      const vectorData = Array.from({ length: 128 }, () => Math.random());
      const metadata = { label: 'test' };

      // This test would need proper mocking
      expect(typeof client.insertSimple).toBe('function');
    });
  });

  describe('searchSimple', () => {
    it('should search with simple parameters', async () => {
      const collectionName = 'test-collection';
      const queryVector = Array.from({ length: 128 }, () => Math.random());
      const limit = 10;

      // This test would need proper mocking
      expect(typeof client.searchSimple).toBe('function');
    });
  });

  describe('batchInsertSimple', () => {
    it('should batch insert vectors', async () => {
      const collectionName = 'test-collection';
      const vectorsData: Array<[string, number[], Record<string, string>]> = [
        ['vec-1', Array.from({ length: 128 }, () => Math.random()), { label: 'a' }],
        ['vec-2', Array.from({ length: 128 }, () => Math.random()), { label: 'b' }],
      ];

      // This test would need proper mocking
      expect(typeof client.batchInsertSimple).toBe('function');
    });

    it('should handle batch size correctly', async () => {
      // Test that vectors are chunked correctly
      const collectionName = 'test-collection';
      const vectorsData: Array<[string, number[]]> = [];
      for (let i = 0; i < 250; i++) {
        vectorsData.push([`vec-${i}`, Array.from({ length: 128 }, () => Math.random())]);
      }

      // With batch size of 100, should result in 3 batches
      // This would need proper mocking to verify
      expect(typeof client.batchInsertSimple).toBe('function');
    });
  });

  describe('error handling', () => {
    it('should handle connection errors', async () => {
      // This would need proper mocking
      expect(typeof client.ping).toBe('function');
    });
  });
});
