use crate::entities::{Metric, Probe, Sensor};
use crate::events::{AcceleratorDiscovered, DomainEvent, MetricCollected};
use crate::ports::AcceleratorBackend;
use crate::value_objects::*;
use crate::DomainResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Accelerator is the main aggregate root representing an AI compute device
/// It contains sensors, maintains state, and produces metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Accelerator {
    pub id: AcceleratorId,
    pub name: String,
    pub accelerator_type: AcceleratorType,
    pub backend_type: String, // e.g., "nvml", "metal", "tpu"
    pub vendor: String,
    pub model: String,
    pub driver_version: Option<String>,
    pub firmware_version: Option<String>,
    pub serial_number: Option<String>,
    pub pci_info: Option<PciInfo>,
    pub sensors: HashMap<SensorId, Sensor>,
    pub probes: HashMap<ProbeId, Probe>,
    pub discovered_at: Timestamp,
    pub last_seen: Timestamp,
    pub enabled: bool,
    pub labels: Labels,
    /// Uncommitted domain events
    #[serde(skip)]
    pub events: Vec<DomainEvent>,
}

/// PCI device information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PciInfo {
    pub bus_id: String,
    pub domain: u32,
    pub bus: u32,
    pub device: u32,
    pub function: u32,
}

impl Accelerator {
    pub fn new(
        name: impl Into<String>,
        accelerator_type: AcceleratorType,
        backend_type: impl Into<String>,
        vendor: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        let id = AcceleratorId::new();
        let now = Timestamp::now();
        let accel_type_for_event = accelerator_type.clone();
        let mut accelerator = Self {
            id,
            name: name.into(),
            accelerator_type,
            backend_type: backend_type.into(),
            vendor: vendor.into(),
            model: model.into(),
            driver_version: None,
            firmware_version: None,
            serial_number: None,
            pci_info: None,
            sensors: HashMap::new(),
            probes: HashMap::new(),
            discovered_at: now,
            last_seen: now,
            enabled: true,
            labels: Labels::new(),
            events: Vec::new(),
        };

        // Emit discovery event
        accelerator
            .events
            .push(DomainEvent::AcceleratorDiscovered(AcceleratorDiscovered {
                accelerator_id: id,
                accelerator_type: accel_type_for_event,
                name: accelerator.name.clone(),
                vendor: accelerator.vendor.clone(),
                model: accelerator.model.clone(),
                timestamp: now,
            }));

        accelerator
    }

    pub fn with_pci_info(mut self, pci_info: PciInfo) -> Self {
        self.pci_info = Some(pci_info);
        self
    }

    pub fn with_driver_version(mut self, version: impl Into<String>) -> Self {
        self.driver_version = Some(version.into());
        self
    }

    pub fn with_firmware_version(mut self, version: impl Into<String>) -> Self {
        self.firmware_version = Some(version.into());
        self
    }

    pub fn with_serial_number(mut self, serial: impl Into<String>) -> Self {
        self.serial_number = Some(serial.into());
        self
    }

    pub fn with_labels(mut self, labels: Labels) -> Self {
        self.labels = labels;
        self
    }

    pub fn add_sensor(&mut self, sensor: Sensor) -> DomainResult<SensorId> {
        if !self.enabled {
            return Err(crate::DomainError::BackendError(
                "Cannot add sensor to disabled accelerator".to_string(),
            ));
        }

        let id = sensor.id;
        self.sensors.insert(id, sensor);
        Ok(id)
    }

    pub fn remove_sensor(&mut self, sensor_id: SensorId) -> DomainResult<()> {
        self.sensors
            .remove(&sensor_id)
            .ok_or_else(|| crate::DomainError::SensorNotFound(sensor_id.to_string()))?;
        Ok(())
    }

    pub fn add_probe(&mut self, probe: Probe) -> DomainResult<ProbeId> {
        let id = probe.id;
        self.probes.insert(id, probe);
        Ok(id)
    }

    pub fn get_sensor(&self, sensor_id: SensorId) -> Option<&Sensor> {
        self.sensors.get(&sensor_id)
    }

    pub fn get_enabled_sensors(&self) -> Vec<&Sensor> {
        self.sensors.values().filter(|s| s.enabled).collect()
    }

    pub fn record_metric(&mut self, metric: Metric) {
        self.last_seen = Timestamp::now();

        // Emit collection event
        self.events
            .push(DomainEvent::MetricCollected(MetricCollected {
                metric_id: metric.id,
                sensor_id: metric.sensor_id,
                accelerator_id: self.id,
                metric_type: metric.metric_type,
                timestamp: metric.timestamp,
            }));
    }

    pub fn update_last_seen(&mut self) {
        self.last_seen = Timestamp::now();
    }

    pub fn mark_unhealthy(&mut self) {
        self.enabled = false;
    }

    pub fn mark_healthy(&mut self) {
        self.enabled = true;
        self.update_last_seen();
    }

    pub fn take_events(&mut self) -> Vec<DomainEvent> {
        std::mem::take(&mut self.events)
    }
}

/// A Sample is a point-in-time snapshot of all metrics from an accelerator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sample {
    pub id: SampleId,
    pub accelerator_id: AcceleratorId,
    pub timestamp: Timestamp,
    pub metrics: Vec<Metric>,
    pub duration_nanos: u64, // Time taken to collect
    pub labels: Labels,
}

impl Sample {
    pub fn new(accelerator_id: AcceleratorId, metrics: Vec<Metric>) -> Self {
        Self {
            id: SampleId::new(),
            accelerator_id,
            timestamp: Timestamp::now(),
            metrics,
            duration_nanos: 0,
            labels: Labels::new(),
        }
    }

    pub fn with_duration(mut self, nanos: u64) -> Self {
        self.duration_nanos = nanos;
        self
    }

    pub fn get_metric(&self, metric_type: MetricType) -> Option<&Metric> {
        self.metrics.iter().find(|m| m.metric_type == metric_type)
    }

    pub fn utilization(&self) -> Option<Utilization> {
        self.get_metric(MetricType::Utilization)
            .and_then(|m| m.value.as_f64())
            .and_then(|v| Utilization::new(v))
    }

    pub fn memory_stats(&self) -> Option<MemoryStats> {
        self.get_metric(MetricType::MemoryUsed)
            .and_then(|m| match &m.value {
                crate::entities::MetricValue::Memory(stats) => Some(*stats),
                _ => None,
            })
    }
}

/// MetricStream represents a continuous collection of samples over time
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MetricStream {
    pub accelerator_id: AcceleratorId,
    pub sensor_id: SensorId,
    pub buffer: Vec<Metric>,
    pub capacity: usize,
    pub sampling_interval_ms: u64,
}

impl MetricStream {
    pub fn new(accelerator_id: AcceleratorId, sensor_id: SensorId, capacity: usize) -> Self {
        Self {
            accelerator_id,
            sensor_id,
            buffer: Vec::with_capacity(capacity),
            capacity,
            sampling_interval_ms: 1000,
        }
    }

    pub fn push(&mut self, metric: Metric) {
        if self.buffer.len() >= self.capacity {
            self.buffer.remove(0);
        }
        self.buffer.push(metric);
    }

    pub fn latest(&self) -> Option<&Metric> {
        self.buffer.last()
    }

    pub fn window(&self, count: usize) -> &[Metric] {
        let start = self.buffer.len().saturating_sub(count);
        &self.buffer[start..]
    }

    pub fn average(&self, count: usize) -> Option<f64> {
        let window = self.window(count);
        if window.is_empty() {
            return None;
        }
        let sum: f64 = window.iter().filter_map(|m| m.value.as_f64()).sum();
        let count = window.iter().filter(|m| m.value.as_f64().is_some()).count() as f64;
        if count > 0.0 {
            Some(sum / count)
        } else {
            None
        }
    }
}
