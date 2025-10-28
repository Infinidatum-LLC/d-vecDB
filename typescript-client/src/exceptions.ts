/**
 * Base exception for all d-vecDB errors
 */
export class VectorDBError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'VectorDBError';
    Object.setPrototypeOf(this, VectorDBError.prototype);
  }
}

/**
 * Connection error
 */
export class ConnectionError extends VectorDBError {
  constructor(message: string) {
    super(message);
    this.name = 'ConnectionError';
    Object.setPrototypeOf(this, ConnectionError.prototype);
  }
}

/**
 * Timeout error
 */
export class TimeoutError extends VectorDBError {
  constructor(message: string) {
    super(message);
    this.name = 'TimeoutError';
    Object.setPrototypeOf(this, TimeoutError.prototype);
  }
}

/**
 * Authentication error
 */
export class AuthenticationError extends VectorDBError {
  constructor(message: string) {
    super(message);
    this.name = 'AuthenticationError';
    Object.setPrototypeOf(this, AuthenticationError.prototype);
  }
}

/**
 * Authorization error
 */
export class AuthorizationError extends VectorDBError {
  constructor(message: string) {
    super(message);
    this.name = 'AuthorizationError';
    Object.setPrototypeOf(this, AuthorizationError.prototype);
  }
}

/**
 * Collection not found error
 */
export class CollectionNotFoundError extends VectorDBError {
  constructor(collectionName: string) {
    super(`Collection '${collectionName}' not found`);
    this.name = 'CollectionNotFoundError';
    Object.setPrototypeOf(this, CollectionNotFoundError.prototype);
  }
}

/**
 * Collection already exists error
 */
export class CollectionExistsError extends VectorDBError {
  constructor(collectionName: string) {
    super(`Collection '${collectionName}' already exists`);
    this.name = 'CollectionExistsError';
    Object.setPrototypeOf(this, CollectionExistsError.prototype);
  }
}

/**
 * Vector not found error
 */
export class VectorNotFoundError extends VectorDBError {
  constructor(vectorId: string) {
    super(`Vector '${vectorId}' not found`);
    this.name = 'VectorNotFoundError';
    Object.setPrototypeOf(this, VectorNotFoundError.prototype);
  }
}

/**
 * Invalid parameter error
 */
export class InvalidParameterError extends VectorDBError {
  constructor(message: string) {
    super(message);
    this.name = 'InvalidParameterError';
    Object.setPrototypeOf(this, InvalidParameterError.prototype);
  }
}

/**
 * Validation error
 */
export class ValidationError extends VectorDBError {
  constructor(message: string) {
    super(message);
    this.name = 'ValidationError';
    Object.setPrototypeOf(this, ValidationError.prototype);
  }
}

/**
 * Server error
 */
export class ServerError extends VectorDBError {
  constructor(message: string, public statusCode?: number) {
    super(message);
    this.name = 'ServerError';
    Object.setPrototypeOf(this, ServerError.prototype);
  }
}

/**
 * Rate limit error
 */
export class RateLimitError extends VectorDBError {
  constructor(message: string) {
    super(message);
    this.name = 'RateLimitError';
    Object.setPrototypeOf(this, RateLimitError.prototype);
  }
}

/**
 * Quota exceeded error
 */
export class QuotaExceededError extends VectorDBError {
  constructor(message: string) {
    super(message);
    this.name = 'QuotaExceededError';
    Object.setPrototypeOf(this, QuotaExceededError.prototype);
  }
}

/**
 * Protocol error
 */
export class ProtocolError extends VectorDBError {
  constructor(message: string) {
    super(message);
    this.name = 'ProtocolError';
    Object.setPrototypeOf(this, ProtocolError.prototype);
  }
}
