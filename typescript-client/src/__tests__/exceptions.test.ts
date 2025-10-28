import {
  VectorDBError,
  ConnectionError,
  CollectionNotFoundError,
  VectorNotFoundError,
  InvalidParameterError,
} from '../exceptions';

describe('Exceptions', () => {
  describe('VectorDBError', () => {
    it('should create a base error', () => {
      const error = new VectorDBError('Test error');
      expect(error).toBeInstanceOf(Error);
      expect(error).toBeInstanceOf(VectorDBError);
      expect(error.message).toBe('Test error');
      expect(error.name).toBe('VectorDBError');
    });
  });

  describe('ConnectionError', () => {
    it('should create a connection error', () => {
      const error = new ConnectionError('Connection failed');
      expect(error).toBeInstanceOf(VectorDBError);
      expect(error).toBeInstanceOf(ConnectionError);
      expect(error.message).toBe('Connection failed');
      expect(error.name).toBe('ConnectionError');
    });
  });

  describe('CollectionNotFoundError', () => {
    it('should create collection not found error', () => {
      const error = new CollectionNotFoundError('my-collection');
      expect(error).toBeInstanceOf(VectorDBError);
      expect(error.message).toContain('my-collection');
      expect(error.message).toContain('not found');
      expect(error.name).toBe('CollectionNotFoundError');
    });
  });

  describe('VectorNotFoundError', () => {
    it('should create vector not found error', () => {
      const error = new VectorNotFoundError('vec-123');
      expect(error).toBeInstanceOf(VectorDBError);
      expect(error.message).toContain('vec-123');
      expect(error.message).toContain('not found');
      expect(error.name).toBe('VectorNotFoundError');
    });
  });

  describe('InvalidParameterError', () => {
    it('should create invalid parameter error', () => {
      const error = new InvalidParameterError('Invalid dimension');
      expect(error).toBeInstanceOf(VectorDBError);
      expect(error.message).toBe('Invalid dimension');
      expect(error.name).toBe('InvalidParameterError');
    });
  });
});
