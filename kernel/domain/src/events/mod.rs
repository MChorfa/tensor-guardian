use crate::value_objects::{AcceleratorId, MetricId, MetricType, ProbeId, SensorId, Timestamp};
use serde::{Deserialize, Serialize};

/// Domain events represent significant occurrences in the domain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DomainEvent {
    /// An accelerator was discovered
    AcceleratorDiscovered(AcceleratorDiscovered),
    /// A metric was collected
    MetricCollected(MetricCollected),
    /// A probe was attached
    ProbeAttached(ProbeAttached),
    /// A probe was detached
    ProbeDetached(ProbeDetached),
    /// A sensor was enabled
    SensorEnabled(SensorEnabled),
    /// A sensor was disabled
    SensorDisabled(SensorDisabled),
    /// An error occurred
    ErrorOccurred(ErrorOccurred),
}

impl DomainEvent {
    pub fn timestamp(&self) -> Timestamp {
        match self {
            DomainEvent::AcceleratorDiscovered(e) => e.timestamp,
            DomainEvent::MetricCollected(e) => e.timestamp,
            DomainEvent::ProbeAttached(e) => e.timestamp,
            DomainEvent::ProbeDetached(e) => e.timestamp,
            DomainEvent::SensorEnabled(e) => e.timestamp,
            DomainEvent::SensorDisabled(e) => e.timestamp,
            DomainEvent::ErrorOccurred(e) => e.timestamp,
        }
    }

    pub fn event_type(&self) -> &'static str {
        match self {
            DomainEvent::AcceleratorDiscovered(_) => "accelerator_discovered",
            DomainEvent::MetricCollected(_) => "metric_collected",
            DomainEvent::ProbeAttached(_) => "probe_attached",
            DomainEvent::ProbeDetached(_) => "probe_detached",
            DomainEvent::SensorEnabled(_) => "sensor_enabled",
            DomainEvent::SensorDisabled(_) => "sensor_disabled",
            DomainEvent::ErrorOccurred(_) => "error_occurred",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcceleratorDiscovered {
    pub accelerator_id: AcceleratorId,
    pub accelerator_type: crate::value_objects::AcceleratorType,
    pub name: String,
    pub vendor: String,
    pub model: String,
    pub timestamp: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricCollected {
    pub metric_id: MetricId,
    pub sensor_id: SensorId,
    pub accelerator_id: AcceleratorId,
    pub metric_type: MetricType,
    pub timestamp: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeAttached {
    pub probe_id: ProbeId,
    pub target: String,
    pub timestamp: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeDetached {
    pub probe_id: ProbeId,
    pub timestamp: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorEnabled {
    pub sensor_id: SensorId,
    pub accelerator_id: AcceleratorId,
    pub timestamp: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorDisabled {
    pub sensor_id: SensorId,
    pub accelerator_id: AcceleratorId,
    pub timestamp: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorOccurred {
    pub source: String,
    pub error_type: ErrorType,
    pub message: String,
    pub timestamp: Timestamp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorType {
    BackendFailure,
    CollectionFailure,
    ProbeAttachFailure,
    CommunicationFailure,
    ConfigurationError,
}
