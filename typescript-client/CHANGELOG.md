# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.2] - 2024-10-28

### Fixed
- **Bug #2: createCollectionSimple missing vector_type**
  - Added `vectorType` parameter with default value of `VectorType.FLOAT32`
  - Fixes HTTP 422 error when creating collections
  - Now sends all required fields to server
  - Backward compatible: existing code will use FLOAT32 default

### Changed
- Updated `createCollectionSimple` signature to include optional `vectorType` parameter

## [0.1.1] - 2024-10-28

### Fixed
- **Critical bug fix**: Fixed response format handling in REST client
  - Server returns responses wrapped in `{"success": true, "data": <actual_data>, "error": null}`
  - Client was incorrectly trying to access nested properties directly
  - Added `unwrapResponse()` method to properly extract data from server responses
  - Affected all API endpoints: collections, vectors, search, and server stats

### Changed
- Improved error handling to detect and throw errors from server response wrapper

## [0.1.0] - 2024-10-28

### Added
- Initial release of d-vecDB TypeScript/JavaScript client
- Full REST API support for d-vecDB server
- Collection management (create, list, get, delete)
- Vector operations (insert, batch insert, get, update, delete)
- Similarity search with HNSW parameters
- Server health and statistics endpoints
- Comprehensive TypeScript type definitions
- Custom exception hierarchy for better error handling
- Simple and advanced API methods
- Examples for basic usage and advanced search
- Full documentation in README
