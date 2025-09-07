"""
Synchronous gRPC client for d-vecDB.
"""

from typing import List, Optional, Dict, Any
import grpc

from ..types import (
    CollectionConfig, Vector, QueryResult, SearchRequest, SearchResponse,
    CollectionStats, ServerStats, HealthResponse, InsertResponse,
    ListCollectionsResponse, CollectionResponse, VectorData, DistanceMetric,
    VectorType, IndexConfig
)
from ..exceptions import (
    VectorDBError, ConnectionError, create_exception_from_grpc_error
)

# Import generated protobuf stubs (would be generated from .proto files)
try:
    from . import vectordb_pb2
    from . import vectordb_pb2_grpc
except ImportError:
    # Create mock classes for development
    class MockMessage:
        def __init__(self, **kwargs):
            for k, v in kwargs.items():
                setattr(self, k, v)
    
    class vectordb_pb2:
        CreateCollectionRequest = MockMessage
        CreateCollectionResponse = MockMessage
        DeleteCollectionRequest = MockMessage
        DeleteCollectionResponse = MockMessage
        ListCollectionsRequest = MockMessage
        ListCollectionsResponse = MockMessage
        GetCollectionInfoRequest = MockMessage
        GetCollectionInfoResponse = MockMessage
        InsertRequest = MockMessage
        InsertResponse = MockMessage
        BatchInsertRequest = MockMessage
        BatchInsertResponse = MockMessage
        DeleteRequest = MockMessage
        DeleteResponse = MockMessage
        QueryRequest = MockMessage
        QueryResponse = MockMessage
        UpdateRequest = MockMessage
        UpdateResponse = MockMessage
        GetStatsRequest = MockMessage
        GetStatsResponse = MockMessage
        HealthRequest = MockMessage
        HealthResponse = MockMessage
        CollectionConfig = MockMessage
        IndexConfig = MockMessage
        Vector = MockMessage
        QueryResult = MockMessage
        CollectionStats = MockMessage
        ServerStats = MockMessage
    
    class MockStub:
        def __init__(self, channel):
            self.channel = channel
        
        def CreateCollection(self, request):
            raise NotImplementedError("gRPC protobuf files not generated")
        
        def DeleteCollection(self, request):
            raise NotImplementedError("gRPC protobuf files not generated")
            
        def ListCollections(self, request):
            raise NotImplementedError("gRPC protobuf files not generated")
            
        def GetCollectionInfo(self, request):
            raise NotImplementedError("gRPC protobuf files not generated")
            
        def Insert(self, request):
            raise NotImplementedError("gRPC protobuf files not generated")
            
        def BatchInsert(self, request):
            raise NotImplementedError("gRPC protobuf files not generated")
            
        def Delete(self, request):
            raise NotImplementedError("gRPC protobuf files not generated")
            
        def Query(self, request):
            raise NotImplementedError("gRPC protobuf files not generated")
            
        def Update(self, request):
            raise NotImplementedError("gRPC protobuf files not generated")
            
        def GetStats(self, request):
            raise NotImplementedError("gRPC protobuf files not generated")
            
        def Health(self, request):
            raise NotImplementedError("gRPC protobuf files not generated")
    
    class vectordb_pb2_grpc:
        VectorDbStub = MockStub


class GrpcClient:
    """Synchronous gRPC client for VectorDB-RS."""
    
    def __init__(
        self,
        host: str = "localhost",
        port: int = 9090,
        ssl: bool = False,
        credentials: Optional[grpc.ChannelCredentials] = None,
        options: Optional[List[tuple]] = None,
        timeout: float = 30.0
    ):
        """
        Initialize gRPC client.
        
        Args:
            host: Server hostname or IP address
            port: Server port number
            ssl: Use secure channel if True
            credentials: gRPC channel credentials
            options: gRPC channel options
            timeout: Default request timeout in seconds
        """
        self.host = host
        self.port = port
        self.ssl = ssl
        self.timeout = timeout
        
        # Build server address
        self.address = f"{host}:{port}"
        
        # Setup gRPC channel
        if ssl or credentials:
            if credentials is None:
                credentials = grpc.ssl_channel_credentials()
            self.channel = grpc.secure_channel(
                self.address, 
                credentials,
                options=options
            )
        else:
            self.channel = grpc.insecure_channel(
                self.address,
                options=options
            )
        
        # Create stub
        self.stub = vectordb_pb2_grpc.VectorDbStub(self.channel)
    
    def __enter__(self):
        """Context manager entry."""
        return self
    
    def __exit__(self, exc_type, exc_val, exc_tb):
        """Context manager exit."""
        self.close()
    
    def close(self):
        """Close the gRPC channel."""
        if hasattr(self, 'channel'):
            self.channel.close()
    
    def _convert_distance_metric(self, metric: DistanceMetric) -> int:
        """Convert Python distance metric to protobuf enum."""
        metric_map = {
            DistanceMetric.COSINE: 1,
            DistanceMetric.EUCLIDEAN: 2,
            DistanceMetric.DOT_PRODUCT: 3,
            DistanceMetric.MANHATTAN: 4,
        }
        return metric_map.get(metric, 1)  # Default to cosine
    
    def _convert_vector_type(self, vtype: VectorType) -> int:
        """Convert Python vector type to protobuf enum."""
        type_map = {
            VectorType.FLOAT32: 1,
            VectorType.FLOAT16: 2,
            VectorType.INT8: 3,
        }
        return type_map.get(vtype, 1)  # Default to float32
    
    def _make_collection_config_proto(self, config: CollectionConfig) -> vectordb_pb2.CollectionConfig:
        """Convert Python CollectionConfig to protobuf."""
        index_config = None
        if config.index_config:
            index_config = vectordb_pb2.IndexConfig(
                max_connections=config.index_config.max_connections,
                ef_construction=config.index_config.ef_construction,
                ef_search=config.index_config.ef_search,
                max_layer=config.index_config.max_layer
            )
        
        return vectordb_pb2.CollectionConfig(
            name=config.name,
            dimension=config.dimension,
            distance_metric=self._convert_distance_metric(config.distance_metric),
            vector_type=self._convert_vector_type(config.vector_type),
            index_config=index_config
        )
    
    def _make_vector_proto(self, vector: Vector) -> vectordb_pb2.Vector:
        """Convert Python Vector to protobuf."""
        metadata = {}
        if vector.metadata:
            # Convert all metadata values to strings for protobuf
            metadata = {k: str(v) for k, v in vector.metadata.items()}
        
        return vectordb_pb2.Vector(
            id=vector.id,
            data=vector.data,
            metadata=metadata
        )
    
    def _convert_query_result(self, proto_result) -> QueryResult:
        """Convert protobuf QueryResult to Python."""
        metadata = dict(proto_result.metadata) if proto_result.metadata else None
        return QueryResult(
            id=proto_result.id,
            distance=proto_result.distance,
            metadata=metadata
        )
    
    # Collection Management
    def create_collection(self, config: CollectionConfig) -> CollectionResponse:
        """Create a new vector collection."""
        try:
            request = vectordb_pb2.CreateCollectionRequest(
                config=self._make_collection_config_proto(config)
            )
            
            response = self.stub.CreateCollection(request, timeout=self.timeout)
            
            return CollectionResponse(
                success=response.success,
                message=response.message
            )
            
        except grpc.RpcError as e:
            raise create_exception_from_grpc_error(e)
    
    def delete_collection(self, name: str) -> CollectionResponse:
        """Delete a vector collection."""
        try:
            request = vectordb_pb2.DeleteCollectionRequest(collection_name=name)
            response = self.stub.DeleteCollection(request, timeout=self.timeout)
            
            return CollectionResponse(
                success=response.success,
                message=response.message
            )
            
        except grpc.RpcError as e:
            raise create_exception_from_grpc_error(e)
    
    def get_collection(self, name: str) -> CollectionResponse:
        """Get collection information."""
        try:
            request = vectordb_pb2.GetCollectionInfoRequest(collection_name=name)
            response = self.stub.GetCollectionInfo(request, timeout=self.timeout)
            
            # Convert protobuf response to Python objects
            # (Implementation would depend on actual protobuf structure)
            return CollectionResponse(success=True)
            
        except grpc.RpcError as e:
            raise create_exception_from_grpc_error(e)
    
    def list_collections(self) -> ListCollectionsResponse:
        """List all collections."""
        try:
            request = vectordb_pb2.ListCollectionsRequest()
            response = self.stub.ListCollections(request, timeout=self.timeout)
            
            return ListCollectionsResponse(
                success=True,
                collections=list(response.collection_names)
            )
            
        except grpc.RpcError as e:
            raise create_exception_from_grpc_error(e)
    
    # Vector Operations
    def insert_vector(self, collection_name: str, vector: Vector) -> InsertResponse:
        """Insert a single vector."""
        try:
            request = vectordb_pb2.InsertRequest(
                collection_name=collection_name,
                vector=self._make_vector_proto(vector)
            )
            
            response = self.stub.Insert(request, timeout=self.timeout)
            
            return InsertResponse(
                success=response.success,
                message=response.message
            )
            
        except grpc.RpcError as e:
            raise create_exception_from_grpc_error(e)
    
    def insert_vectors(self, collection_name: str, vectors: List[Vector]) -> InsertResponse:
        """Insert multiple vectors."""
        try:
            proto_vectors = [self._make_vector_proto(v) for v in vectors]
            request = vectordb_pb2.BatchInsertRequest(
                collection_name=collection_name,
                vectors=proto_vectors
            )
            
            response = self.stub.BatchInsert(request, timeout=self.timeout)
            
            return InsertResponse(
                success=response.success,
                message=response.message,
                inserted_count=getattr(response, 'inserted_count', len(vectors))
            )
            
        except grpc.RpcError as e:
            raise create_exception_from_grpc_error(e)
    
    def delete_vector(self, collection_name: str, vector_id: str) -> InsertResponse:
        """Delete a vector by ID."""
        try:
            request = vectordb_pb2.DeleteRequest(
                collection_name=collection_name,
                vector_id=vector_id
            )
            
            response = self.stub.Delete(request, timeout=self.timeout)
            
            return InsertResponse(
                success=response.success,
                message=response.message
            )
            
        except grpc.RpcError as e:
            raise create_exception_from_grpc_error(e)
    
    # Search Operations
    def search(
        self, 
        collection_name: str,
        query_vector: VectorData,
        limit: int = 10,
        ef_search: Optional[int] = None,
        filter: Optional[Dict[str, Any]] = None
    ) -> SearchResponse:
        """Search for similar vectors."""
        try:
            if hasattr(query_vector, 'tolist'):
                query_vector = query_vector.tolist()
            
            # Convert filter to protobuf map format
            proto_filter = {}
            if filter:
                proto_filter = {k: str(v) for k, v in filter.items()}
            
            request = vectordb_pb2.QueryRequest(
                collection_name=collection_name,
                query_vector=query_vector,
                limit=limit,
                ef_search=ef_search,
                filter=proto_filter
            )
            
            response = self.stub.Query(request, timeout=self.timeout)
            
            results = [self._convert_query_result(r) for r in response.results]
            
            return SearchResponse(
                success=True,
                results=results,
                query_time_ms=getattr(response, 'query_time_ms', None)
            )
            
        except grpc.RpcError as e:
            raise create_exception_from_grpc_error(e)
    
    # Server Operations
    def get_server_stats(self) -> ServerStats:
        """Get server statistics."""
        try:
            request = vectordb_pb2.GetStatsRequest()
            response = self.stub.GetStats(request, timeout=self.timeout)
            
            stats = response.stats
            return ServerStats(
                total_vectors=stats.total_vectors,
                total_collections=stats.total_collections,
                memory_usage=stats.memory_usage,
                disk_usage=stats.disk_usage,
                uptime_seconds=stats.uptime_seconds
            )
            
        except grpc.RpcError as e:
            raise create_exception_from_grpc_error(e)
    
    def health_check(self) -> HealthResponse:
        """Check server health."""
        try:
            request = vectordb_pb2.HealthRequest()
            response = self.stub.Health(request, timeout=self.timeout)
            
            return HealthResponse(
                success=response.healthy,
                data="OK" if response.healthy else "NOT_OK",
                error=None
            )
            
        except grpc.RpcError as e:
            raise create_exception_from_grpc_error(e)
    
    # Convenience methods
    def ping(self) -> bool:
        """Check if server is reachable."""
        try:
            self.health_check()
            return True
        except VectorDBError:
            return False