use crate::aggregates::{Accelerator, Sample};
use crate::entities::{Metric, Probe, Sensor};
use crate::value_objects::{AcceleratorId, AcceleratorType, Labels, SensorId};
use crate::DomainResult;
use async_trait::async_trait;
use std::collections::HashMap;
use std::fmt;

/// Port for accelerator backends (NVIDIA, Metal, TPU, etc.)
/// This is the primary port in the hexagonal architecture
#[async_trait]
pub trait AcceleratorBackend: Send + Sync + fmt::Debug {
    /// Unique identifier for this backend type
    fn backend_type(&self) -> &'static str;
    
    /// Supported accelerator types
    fn supported_types(&self) -> Vec<AcceleratorType>;
    
    /// Discover all available accelerators
    async fn discover(&self) -> DomainResult<Vec<Accelerator>>;
    
    /// Collect metrics from a specific accelerator
    async fn collect(&self, accelerator: &Accelerator) -> DomainResult<Sample>;
    
    /// Check if backend is healthy
    async fn health(&self) -> BackendHealth;
    
    /// Get backend version info
    fn version(&self) -> BackendVersion;
}

/// Port for sensor backends (sysfs, hwmon, etc.)
#[async_trait]
pub trait SensorBackend: Send + Sync + fmt::Debug {
    fn backend_type(&self) -> &'static str;
    
    /// Discover sensors for a given accelerator
    async fn discover_sensors(
        &self,
        accelerator_id: AcceleratorId,
    ) -> DomainResult<Vec<Sensor>>;
    
    /// Read a single sensor value
    async fn read(&self, sensor: &Sensor) -> DomainResult<Metric>;
}

/// Port for eBPF probe backends
#[async_trait]
pub trait ProbeBackend: Send + Sync + fmt::Debug {
    fn backend_type(&self) -> &'static str;
    
    /// Load and attach a probe
    async fn attach(&self, probe: &mut Probe) -> DomainResult<()>;
    
    /// Detach a probe
    async fn detach(&self, probe: &mut Probe) -> DomainResult<()>;
    
    /// Poll for probe events
    async fn poll(&self) -> DomainResult<Vec<ProbeEvent>>;
}

/// Event produced by eBPF probes
#[derive(Debug, Clone)]
pub struct ProbeEvent {
    pub probe_id: crate::value_objects::ProbeId,
    pub timestamp: crate::value_objects::Timestamp,
    pub data: Vec<u8>,
    pub event_type: String,
}

/// Health status of a backend
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendHealth {
    Healthy,
    Degraded { reason: &'static str },
    Unhealthy { reason: &'static str },
}

impl BackendHealth {
    pub fn is_healthy(&self) -> bool {
        matches!(self, BackendHealth::Healthy)
    }
}

/// Version information for backends
#[derive(Debug, Clone)]
pub struct BackendVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub git_hash: Option<String>,
    pub build_date: Option<String>,
}

impl fmt::Display for BackendVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if let Some(hash) = &self.git_hash {
            write!(f, "-{}", &hash[..8.min(hash.len())])?;
        }
        Ok(())
    }
}

/// Factory for creating backends based on platform
pub trait BackendFactory: Send + Sync {
    /// Create all available backends for the current platform
    fn create_backends(&self) -> Vec<Box<dyn AcceleratorBackend>>;
    
    /// Check which backends are available on this platform
    fn available_backends(&self) -> Vec<&'static str>;
}

/// Repository port for accelerator persistence
#[async_trait]
pub trait AcceleratorRepository: Send + Sync {
    async fn save(&self, accelerator: &Accelerator) -> DomainResult<()>;
    async fn find_by_id(&self, id: AcceleratorId) -> DomainResult<Option<Accelerator>>;
    async fn find_all(&self) -> DomainResult<Vec<Accelerator>>;
    async fn find_by_type(&self, accel_type: AcceleratorType) -> DomainResult<Vec<Accelerator>>;
    async fn delete(&self, id: AcceleratorId) -> DomainResult<()>;
}

/// Repository port for metric persistence
#[async_trait]
pub trait MetricRepository: Send + Sync {
    async fn save(&self, metric: &Metric) -> DomainResult<()>;
    async fn save_batch(&self, metrics: &[Metric]) -> DomainResult<()>;
    async fn query(
        &self,
        accelerator_id: AcceleratorId,
        sensor_id: Option<SensorId>,
        start_time: crate::value_objects::Timestamp,
        end_time: crate::value_objects::Timestamp,
        limit: usize,
    ) -> DomainResult<Vec<Metric>>;
}

/// Event bus port for publishing domain events
#[async_trait]
pub trait EventBus: Send + Sync {
    async fn publish(&self, event: crate::events::DomainEvent) -> DomainResult<()>;
}
