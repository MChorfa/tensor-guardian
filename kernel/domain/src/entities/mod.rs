use crate::value_objects::*;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A Sensor represents a data source that produces metrics
/// Sensors are attached to Accelerators and can be:
/// - Hardware sensors (temperature, power, etc.)
/// - Kernel probes (eBPF)
/// - Software counters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sensor {
    pub id: SensorId,
    pub accelerator_id: AcceleratorId,
    pub name: String,
    pub metric_type: MetricType,
    pub unit: Unit,
    pub labels: Labels,
    pub enabled: bool,
    pub created_at: Timestamp,
}

impl Sensor {
    pub fn new(
        accelerator_id: AcceleratorId,
        name: impl Into<String>,
        metric_type: MetricType,
        unit: Unit,
    ) -> Self {
        Self {
            id: SensorId::new(),
            accelerator_id,
            name: name.into(),
            metric_type,
            unit,
            labels: Labels::new(),
            enabled: true,
            created_at: Timestamp::now(),
        }
    }

    pub fn with_labels(mut self, labels: Labels) -> Self {
        self.labels = labels;
        self
    }

    pub fn disable(mut self) -> Self {
        self.enabled = false;
        self
    }
}

/// A Metric represents a single data point collected from a Sensor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
    pub id: MetricId,
    pub sensor_id: SensorId,
    pub accelerator_id: AcceleratorId,
    pub metric_type: MetricType,
    pub value: MetricValue,
    pub unit: Unit,
    pub timestamp: Timestamp,
    pub labels: Labels,
}

/// The value of a metric - can be scalar or compound
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricValue {
    Scalar(f64),
    Integer(u64),
    Memory(MemoryStats),
    Utilization(Utilization),
    Temperature(Temperature),
    Power(Power),
    Frequency(Frequency),
    /// Raw bytes (for custom/kernel data)
    Raw(Vec<u8>),
}

impl MetricValue {
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            MetricValue::Scalar(v) => Some(*v),
            MetricValue::Integer(v) => Some(*v as f64),
            MetricValue::Utilization(u) => Some(u.percent),
            MetricValue::Temperature(t) => Some(t.celsius),
            MetricValue::Power(p) => Some(p.watts),
            MetricValue::Frequency(f) => Some(f.hertz as f64),
            MetricValue::Memory(m) => Some(m.used_bytes as f64),
            MetricValue::Raw(_) => None,
        }
    }

    pub fn as_u64(&self) -> Option<u64> {
        match self {
            MetricValue::Integer(v) => Some(*v),
            MetricValue::Scalar(v) => Some(*v as u64),
            MetricValue::Memory(m) => Some(m.used_bytes),
            MetricValue::Frequency(f) => Some(f.hertz),
            _ => self.as_f64().map(|f| f as u64),
        }
    }
}

impl Metric {
    pub fn new(
        sensor_id: SensorId,
        accelerator_id: AcceleratorId,
        metric_type: MetricType,
        value: MetricValue,
        unit: Unit,
    ) -> Self {
        Self {
            id: MetricId::new(),
            sensor_id,
            accelerator_id,
            metric_type,
            value,
            unit,
            timestamp: Timestamp::now(),
            labels: Labels::new(),
        }
    }

    pub fn with_labels(mut self, labels: Labels) -> Self {
        self.labels = labels;
        self
    }

    pub fn with_timestamp(mut self, timestamp: Timestamp) -> Self {
        self.timestamp = timestamp;
        self
    }
}

/// A Probe represents an eBPF instrumentation point in the kernel
/// Probes attach to kernel functions and emit events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Probe {
    pub id: ProbeId,
    pub name: String,
    pub probe_type: ProbeType,
    pub target: String, // Kernel function or tracepoint
    pub program_type: ProbeProgramType,
    pub enabled: bool,
    pub labels: Labels,
    pub attached_at: Option<Timestamp>,
}

/// Types of eBPF probes
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProbeType {
    /// kprobe - function entry
    Kprobe,
    /// kretprobe - function return
    Kretprobe,
    /// tracepoint - kernel tracepoint
    Tracepoint,
    /// uprobe - userspace function entry
    Uprobe,
    /// uretprobe - userspace function return  
    Uretprobe,
    /// fentry - BPF trampolines (fentry/fexit)
    Fentry,
    /// fexit - BPF trampolines (fentry/fexit)
    Fexit,
    /// tp_btf - BTF-enabled tracepoint
    TpBtf,
    /// raw_tp - raw tracepoint
    RawTracepoint,
}

/// What the probe program produces
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProbeProgramType {
    /// GPU memory allocation events
    GpuMemory,
    /// GPU compute launch events
    GpuCompute,
    /// Memory pressure events
    MemoryPressure,
    /// I/O events (DMA, PCIe)
    IoTrace,
    /// Scheduler events
    Scheduler,
    /// Custom probe type
    Custom { category: String },
}

impl Probe {
    pub fn new(
        name: impl Into<String>,
        probe_type: ProbeType,
        target: impl Into<String>,
        program_type: ProbeProgramType,
    ) -> Self {
        Self {
            id: ProbeId::new(),
            name: name.into(),
            probe_type,
            target: target.into(),
            program_type,
            enabled: false,
            labels: Labels::new(),
            attached_at: None,
        }
    }

    pub fn mark_attached(&mut self) {
        self.enabled = true;
        self.attached_at = Some(Timestamp::now());
    }

    pub fn mark_detached(&mut self) {
        self.enabled = false;
        self.attached_at = None;
    }
}
