pub mod commands;
pub mod queries;
pub mod services;
pub mod policies;

pub use commands::{
    CollectMetrics, CollectMetricsHandler,
    DiscoverAccelerators, DiscoverAcceleratorsHandler,
    AttachProbe, AttachProbeHandler,
    ConfigureSampling, ConfigureSamplingHandler,
};

pub use queries::{
    GetAcceleratorMetrics, GetAcceleratorMetricsHandler,
    ListAccelerators, ListAcceleratorsHandler,
    GetHealthStatus, GetHealthStatusHandler,
};

pub use services::{
    CollectionService, SamplingConfiguration, CollectionStrategy,
};

pub use policies::{
    SamplingPolicy, RetentionPolicy, PolicyEngine,
};

use thiserror::Error;

/// Application layer errors
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Domain error: {0}")]
    Domain(#[from] tensor_guardian_domain::DomainError),
    
    #[error("Backend not available: {0}")]
    BackendNotAvailable(String),
    
    #[error("Collection in progress")]
    CollectionInProgress,
    
    #[error("Configuration error: {0}")]
    Configuration(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type AppResult<T> = Result<T, AppError>;
