pub mod aggregates;
pub mod entities;
pub mod events;
pub mod factory;
pub mod ports;
pub mod services;
pub mod value_objects;

pub use aggregates::{Accelerator, MetricStream, Sample};
pub use entities::{Metric, Probe, Sensor};
pub use events::{AcceleratorDiscovered, DomainEvent, MetricCollected};
pub use factory::PlatformBackendFactory;
pub use ports::{AcceleratorBackend, BackendFactory, BackendHealth, ProbeBackend, SensorBackend};
pub use value_objects::{
    AcceleratorId, AcceleratorType, Frequency, Labels, MemoryStats, MetricId, MetricType, Power,
    ProbeId, SampleId, SensorId, Temperature, Timestamp, Unit, Utilization,
};

use thiserror::Error;

/// Domain-level errors
#[derive(Error, Debug, Clone, PartialEq)]
pub enum DomainError {
    #[error("Accelerator not found: {0}")]
    AcceleratorNotFound(String),

    #[error("Sensor not found: {0}")]
    SensorNotFound(String),

    #[error("Invalid metric value: {0}")]
    InvalidMetricValue(String),

    #[error("Backend error: {0}")]
    BackendError(String),

    #[error("Collection failed: {0}")]
    CollectionFailed(String),

    #[error("Configuration error: {0}")]
    ConfigurationError(String),
}

pub type DomainResult<T> = Result<T, DomainError>;
