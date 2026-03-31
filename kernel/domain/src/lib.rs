pub mod aggregates;
pub mod entities;
pub mod events;
pub mod services;
pub mod value_objects;
pub mod ports;

pub use aggregates::{Accelerator, Sample, MetricStream};
pub use entities::{Sensor, Metric, Probe};
pub use events::{DomainEvent, MetricCollected, AcceleratorDiscovered};
pub use value_objects::{
    AcceleratorId, SensorId, MetricId, SampleId, ProbeId,
    AcceleratorType, MetricType, Unit, Timestamp, Labels,
    Utilization, MemoryStats, Temperature, Power, Frequency,
};
pub use ports::{
    AcceleratorBackend, SensorBackend, ProbeBackend,
    BackendFactory, BackendHealth,
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
