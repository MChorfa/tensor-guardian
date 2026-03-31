//! gRPC/REST API for tensor-guardian
//! 
//! TODO: Implement tonic-based gRPC server and OpenAPI/REST endpoints

use anyhow::Result;

/// API server configuration
pub struct ApiServer;

impl ApiServer {
    pub fn new() -> Self {
        Self
    }

    /// Start the API server
    pub async fn start(&self, _port: u16) -> Result<()> {
        // TODO: Implement gRPC/REST server using tonic and axum
        tracing::info!("API server not yet implemented");
        Ok(())
    }
}

impl Default for ApiServer {
    fn default() -> Self {
        Self::new()
    }
}
