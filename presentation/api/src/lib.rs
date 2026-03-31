//! gRPC/REST API for tensor-guardian
//!
//! Provides dual-protocol API server for accelerator monitoring:
//! - gRPC (port 50051): High-performance streaming for real-time metrics
//! - REST HTTP (port 8080): Simple JSON endpoints for integrations

pub mod error;
pub mod convert;
pub mod server;

pub mod grpc {
    //! gRPC service implementation
    pub mod accelerator;
}

pub mod rest {
    //! REST HTTP handlers
    pub mod handlers;
}

// Include generated protobuf code
pub mod proto {
    #![allow(clippy::all)]
    tonic::include_proto!("tensor_guardian.api.v1");
}

pub use error::{ApiError, Result};
pub use server::ApiServer;

// Re-export generated types for convenience
pub use proto::accelerator_service_client::AcceleratorServiceClient;
pub use proto::accelerator_service_server::{AcceleratorService, AcceleratorServiceServer};
pub use proto::{
    Accelerator, BackendStatus, CollectMetricsRequest, CollectMetricsResponse,
    GetAcceleratorRequest, HealthCheckRequest, HealthCheckResponse, ListAcceleratorsRequest,
    ListAcceleratorsResponse, MetricSample, Sensor, StreamMetricsRequest,
};
