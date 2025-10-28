import { DistanceMetric, VectorType, CollectionConfig, Vector } from '../types';

describe('Types', () => {
  describe('DistanceMetric', () => {
    it('should have all distance metrics', () => {
      expect(DistanceMetric.COSINE).toBe('Cosine');
      expect(DistanceMetric.EUCLIDEAN).toBe('Euclidean');
      expect(DistanceMetric.DOT_PRODUCT).toBe('DotProduct');
      expect(DistanceMetric.MANHATTAN).toBe('Manhattan');
    });
  });

  describe('VectorType', () => {
    it('should have all vector types', () => {
      expect(VectorType.FLOAT32).toBe('Float32');
      expect(VectorType.FLOAT16).toBe('Float16');
      expect(VectorType.INT8).toBe('Int8');
    });
  });

  describe('CollectionConfig', () => {
    it('should accept valid collection config', () => {
      const config: CollectionConfig = {
        name: 'test-collection',
        dimension: 128,
        distanceMetric: DistanceMetric.COSINE,
        vectorType: VectorType.FLOAT32,
        indexConfig: {
          maxConnections: 16,
          efConstruction: 200,
          efSearch: 50,
          maxLayer: 16,
        },
      };

      expect(config.name).toBe('test-collection');
      expect(config.dimension).toBe(128);
      expect(config.distanceMetric).toBe(DistanceMetric.COSINE);
    });
  });

  describe('Vector', () => {
    it('should accept vector with number array', () => {
      const vector: Vector = {
        id: 'vec-1',
        data: [0.1, 0.2, 0.3],
        metadata: { label: 'test' },
      };

      expect(vector.id).toBe('vec-1');
      expect(vector.data).toHaveLength(3);
      expect(vector.metadata?.label).toBe('test');
    });

    it('should accept vector with Float32Array', () => {
      const vector: Vector = {
        id: 'vec-2',
        data: new Float32Array([0.1, 0.2, 0.3]),
      };

      expect(vector.id).toBe('vec-2');
      expect(vector.data).toBeInstanceOf(Float32Array);
    });
  });
});
