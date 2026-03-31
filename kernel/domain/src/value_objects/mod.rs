use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use uuid::Uuid;

/// Unique identifier for accelerators
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AcceleratorId(pub Uuid);

impl AcceleratorId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for AcceleratorId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for AcceleratorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for sensors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SensorId(pub Uuid);

impl SensorId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl fmt::Display for SensorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for SensorId {
    fn default() -> Self {
        Self::new()
    }
}

/// Unique identifier for metrics
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MetricId(pub Uuid);

impl MetricId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for MetricId {
    fn default() -> Self {
        Self::new()
    }
}

/// Unique identifier for samples
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SampleId(pub Uuid);

impl SampleId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for SampleId {
    fn default() -> Self {
        Self::new()
    }
}

/// Unique identifier for probes (eBPF instrumentation points)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProbeId(pub Uuid);

impl ProbeId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ProbeId {
    fn default() -> Self {
        Self::new()
    }
}

/// Types of AI accelerators
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AcceleratorType {
    /// NVIDIA GPU (CUDA)
    NvidiaGpu,
    /// Apple Metal/Neural Engine
    AppleNeuralEngine,
    /// Google Cloud TPU
    GoogleTpu,
    /// AMD GPU (ROCm)
    AmdGpu,
    /// Intel GPU/NA
    IntelNpu,
    /// Custom/user-defined accelerator
    Custom { name: String, vendor: String },
    /// Generic CPU fallback
    Cpu,
}

impl fmt::Display for AcceleratorType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AcceleratorType::NvidiaGpu => write!(f, "nvidia_gpu"),
            AcceleratorType::AppleNeuralEngine => write!(f, "apple_neural_engine"),
            AcceleratorType::GoogleTpu => write!(f, "google_tpu"),
            AcceleratorType::AmdGpu => write!(f, "amd_gpu"),
            AcceleratorType::IntelNpu => write!(f, "intel_npu"),
            AcceleratorType::Custom { name, vendor } => write!(f, "{}_{}", vendor, name),
            AcceleratorType::Cpu => write!(f, "cpu"),
        }
    }
}

/// Types of metrics that can be collected
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MetricType {
    /// Compute utilization percentage (0-100%)
    Utilization,
    /// Memory usage (bytes)
    MemoryUsed,
    /// Total memory (bytes)
    MemoryTotal,
    /// Temperature (Celsius)
    Temperature,
    /// Power consumption (Watts)
    Power,
    /// Clock frequency (MHz)
    Frequency,
    /// Fan speed percentage (0-100%)
    FanSpeed,
    /// Throughput (ops/sec or bytes/sec)
    Throughput,
    /// Latency (microseconds)
    Latency,
    /// Error rate (errors/sec)
    ErrorRate,
    /// Custom metric type
    Custom { name: String },
}

impl fmt::Display for MetricType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MetricType::Utilization => write!(f, "utilization"),
            MetricType::MemoryUsed => write!(f, "memory_used"),
            MetricType::MemoryTotal => write!(f, "memory_total"),
            MetricType::Temperature => write!(f, "temperature"),
            MetricType::Power => write!(f, "power"),
            MetricType::Frequency => write!(f, "frequency"),
            MetricType::FanSpeed => write!(f, "fan_speed"),
            MetricType::Throughput => write!(f, "throughput"),
            MetricType::Latency => write!(f, "latency"),
            MetricType::ErrorRate => write!(f, "error_rate"),
            MetricType::Custom { name } => write!(f, "{}", name),
        }
    }
}

/// Units for metric values
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Unit {
    /// Percentage (0-100)
    Percent,
    /// Bytes
    Bytes,
    /// Kilobytes
    Kilobytes,
    /// Megabytes
    Megabytes,
    /// Gigabytes
    Gigabytes,
    /// Celsius
    Celsius,
    /// Watts
    Watts,
    /// Megahertz
    Megahertz,
    /// Gigahertz
    Gigahertz,
    /// Microseconds
    Microseconds,
    /// Milliseconds
    Milliseconds,
    /// Seconds
    Seconds,
    /// Operations per second
    OpsPerSecond,
    /// Bytes per second
    BytesPerSecond,
    /// Count
    Count,
    /// No unit (dimensionless)
    None,
}

impl fmt::Display for Unit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Unit::Percent => write!(f, "%"),
            Unit::Bytes => write!(f, "B"),
            Unit::Kilobytes => write!(f, "KB"),
            Unit::Megabytes => write!(f, "MB"),
            Unit::Gigabytes => write!(f, "GB"),
            Unit::Celsius => write!(f, "°C"),
            Unit::Watts => write!(f, "W"),
            Unit::Megahertz => write!(f, "MHz"),
            Unit::Gigahertz => write!(f, "GHz"),
            Unit::Microseconds => write!(f, "µs"),
            Unit::Milliseconds => write!(f, "ms"),
            Unit::Seconds => write!(f, "s"),
            Unit::OpsPerSecond => write!(f, "ops/s"),
            Unit::BytesPerSecond => write!(f, "B/s"),
            Unit::Count => write!(f, "count"),
            Unit::None => write!(f, ""),
        }
    }
}

/// Timestamp wrapper with nanosecond precision
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Timestamp {
    pub nanos: i64,
}

impl Timestamp {
    pub fn now() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        Self {
            nanos: duration.as_nanos() as i64,
        }
    }

    pub fn from_nanos(nanos: i64) -> Self {
        Self { nanos }
    }

    pub fn as_seconds(&self) -> f64 {
        self.nanos as f64 / 1_000_000_000.0
    }
}

impl Default for Timestamp {
    fn default() -> Self {
        Self::now()
    }
}

/// Labels for metric attribution
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Labels {
    inner: HashMap<String, String>,
}

impl Labels {
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    pub fn with<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        self.inner.insert(key.into(), value.into());
        self
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.inner.get(key)
    }

    pub fn insert<K, V>(&mut self, key: K, value: V)
    where
        K: Into<String>,
        V: Into<String>,
    {
        self.inner.insert(key.into(), value.into());
    }

    pub fn inner(&self) -> &HashMap<String, String> {
        &self.inner
    }
}

impl From<HashMap<String, String>> for Labels {
    fn from(map: HashMap<String, String>) -> Self {
        Self { inner: map }
    }
}

/// Utilization percentage value object
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Utilization {
    pub percent: f64,
}

impl Utilization {
    pub fn new(percent: f64) -> Option<Self> {
        if percent >= 0.0 && percent <= 100.0 {
            Some(Self { percent })
        } else {
            None
        }
    }

    pub fn as_percent(&self) -> f64 {
        self.percent
    }
}

impl Default for Utilization {
    fn default() -> Self {
        Self { percent: 0.0 }
    }
}

/// Memory statistics value object
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MemoryStats {
    pub used_bytes: u64,
    pub total_bytes: u64,
    pub free_bytes: u64,
}

impl MemoryStats {
    pub fn new(used: u64, total: u64) -> Self {
        Self {
            used_bytes: used,
            total_bytes: total,
            free_bytes: total.saturating_sub(used),
        }
    }

    pub fn utilization(&self) -> Utilization {
        if self.total_bytes == 0 {
            return Utilization::default();
        }
        let percent = (self.used_bytes as f64 / self.total_bytes as f64) * 100.0;
        Utilization::new(percent.min(100.0)).unwrap_or_default()
    }
}

/// Temperature value object
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Temperature {
    pub celsius: f64,
}

impl Temperature {
    pub fn new(celsius: f64) -> Self {
        Self { celsius }
    }

    pub fn from_fahrenheit(f: f64) -> Self {
        Self {
            celsius: (f - 32.0) * 5.0 / 9.0,
        }
    }

    pub fn as_celsius(&self) -> f64 {
        self.celsius
    }

    pub fn as_fahrenheit(&self) -> f64 {
        self.celsius * 9.0 / 5.0 + 32.0
    }
}

/// Power consumption value object
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Power {
    pub watts: f64,
}

impl Power {
    pub fn new(watts: f64) -> Self {
        Self { watts }
    }

    pub fn as_watts(&self) -> f64 {
        self.watts
    }

    pub fn as_milliwatts(&self) -> u64 {
        (self.watts * 1000.0) as u64
    }
}

/// Frequency value object
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Frequency {
    pub hertz: u64,
}

impl Frequency {
    pub fn from_mhz(mhz: u64) -> Self {
        Self {
            hertz: mhz * 1_000_000,
        }
    }

    pub fn from_ghz(ghz: f64) -> Self {
        Self {
            hertz: (ghz * 1_000_000_000.0) as u64,
        }
    }

    pub fn as_hertz(&self) -> u64 {
        self.hertz
    }

    pub fn as_mhz(&self) -> u64 {
        self.hertz / 1_000_000
    }

    pub fn as_ghz(&self) -> f64 {
        self.hertz as f64 / 1_000_000_000.0
    }
}
