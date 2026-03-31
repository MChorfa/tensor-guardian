//! Dual-protocol API server
//!
//! Runs both gRPC and HTTP REST servers concurrently.

use crate::{
    grpc::accelerator::AcceleratorGrpcService,
    proto::accelerator_service_server::AcceleratorServiceServer,
    rest::handlers::{create_router, ApiState},
};
use std::net::SocketAddr;
use tokio::task::JoinHandle;
use tracing::{error, info};

/// API server configuration
pub struct ApiServer {
    grpc_addr: SocketAddr,
    http_addr: SocketAddr,
}

impl ApiServer {
    /// Create new API server
    pub fn new(grpc_addr: SocketAddr, http_addr: SocketAddr) -> Self {
        Self {
            grpc_addr,
            http_addr,
        }
    }

    /// Create with default addresses
    pub fn with_defaults() -> Self {
        Self::new(
            "0.0.0.0:50051".parse().unwrap(),
            "0.0.0.0:8080".parse().unwrap(),
        )
    }

    /// Run both servers
    pub async fn run(self) -> anyhow::Result<()> {
        info!(
            "Starting API servers - gRPC on {}, HTTP on {}",
            self.grpc_addr, self.http_addr
        );

        // Initialize gRPC service
        let grpc_service = AcceleratorGrpcService::new().await?;
        let grpc_server = tonic::transport::Server::builder()
            .add_service(AcceleratorServiceServer::new(grpc_service));

        // Initialize REST router
        let api_state = ApiState::new().await?;
        let rest_router = create_router(api_state);

        // Spawn gRPC server
        let grpc_handle: JoinHandle<anyhow::Result<()>> = tokio::spawn(async move {
            info!("gRPC server starting on {}", self.grpc_addr);
            grpc_server
                .serve(self.grpc_addr)
                .await
                .map_err(|e| anyhow::anyhow!("gRPC server error: {}", e))?;
            Ok(())
        });

        // Spawn HTTP server
        let http_handle: JoinHandle<anyhow::Result<()>> = tokio::spawn(async move {
            info!("HTTP server starting on {}", self.http_addr);
            let listener = tokio::net::TcpListener::bind(self.http_addr).await?;
            axum::serve(listener, rest_router)
                .await
                .map_err(|e| anyhow::anyhow!("HTTP server error: {}", e))?;
            Ok(())
        });

        // Wait for both servers
        tokio::select! {
            result = grpc_handle => {
                if let Err(e) = result {
                    error!("gRPC server task failed: {:?}", e);
                }
            }
            result = http_handle => {
                if let Err(e) = result {
                    error!("HTTP server task failed: {:?}", e);
                }
            }
        }

        info!("API servers stopped");
        Ok(())
    }
}
