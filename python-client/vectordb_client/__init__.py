"""
d-vecDB Python Client

A comprehensive Python API interface for d-vecDB vector database.
Supports both REST and gRPC protocols with synchronous and asynchronous operations.
"""

from .client import VectorDBClient
from .async_client import AsyncVectorDBClient
from .rest.client import RestClient
from .grpc.client import GrpcClient
from .types import (
    CollectionConfig,
    Vector,
    QueryResult,
    SearchRequest,
    DistanceMetric,
    VectorType,
    IndexConfig,
    CollectionStats,
    ServerStats,
)
from .exceptions import (
    VectorDBError,
    ConnectionError,
    CollectionNotFoundError,
    VectorNotFoundError,
    InvalidParameterError,
    ServerError,
)

__version__ = "0.1.1"
__author__ = "d-vecDB Team"
__email__ = "durai@infinidatum.com"

# Main client classes
__all__ = [
    # Main client interfaces
    "VectorDBClient",
    "AsyncVectorDBClient",
    
    # Protocol-specific clients
    "RestClient", 
    "GrpcClient",
    
    # Data types
    "CollectionConfig",
    "Vector",
    "QueryResult", 
    "SearchRequest",
    "DistanceMetric",
    "VectorType",
    "IndexConfig",
    "CollectionStats",
    "ServerStats",
    
    # Exceptions
    "VectorDBError",
    "ConnectionError",
    "CollectionNotFoundError", 
    "VectorNotFoundError",
    "InvalidParameterError",
    "ServerError",
    
    # Metadata
    "__version__",
    "__author__",
    "__email__",
]

# Convenience functions
def connect(
    host: str = "localhost",
    port: int = 8080,
    protocol: str = "rest",
    **kwargs
) -> VectorDBClient:
    """
    Create a VectorDB client connection.
    
    Args:
        host: Server hostname or IP address
        port: Server port number
        protocol: Protocol to use ("rest" or "grpc")
        **kwargs: Additional connection parameters
        
    Returns:
        VectorDBClient instance
        
    Example:
        >>> client = connect("localhost", 8080, "rest")
        >>> collections = client.list_collections()
    """
    return VectorDBClient(host=host, port=port, protocol=protocol, **kwargs)


async def aconnect(
    host: str = "localhost", 
    port: int = 8080,
    protocol: str = "rest",
    **kwargs
) -> AsyncVectorDBClient:
    """
    Create an async VectorDB client connection.
    
    Args:
        host: Server hostname or IP address
        port: Server port number  
        protocol: Protocol to use ("rest" or "grpc")
        **kwargs: Additional connection parameters
        
    Returns:
        AsyncVectorDBClient instance
        
    Example:
        >>> client = await aconnect("localhost", 8080, "rest")
        >>> collections = await client.list_collections()
    """
    client = AsyncVectorDBClient(host=host, port=port, protocol=protocol, **kwargs)
    await client.connect()
    return client