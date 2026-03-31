//! Domain <-> Protobuf conversions
//!
//! Converts between tensor-guardian domain types and protobuf generated types.

use crate::proto;
use tensor_guardian_domain::{
    aggregates::{Accelerator, Sample},
    entities::{Metric, Sensor},
    value_objects::{
        AcceleratorId, AcceleratorType, MetricId, MetricType, SensorId, Timestamp, Unit,
    },
};
use std::collections::HashMap;

// ============== Accelerator conversions ==============

pub fn accelerator_to_proto(accel: &Accelerator) -> proto::Accelerator {
    proto::Accelerator {
        id: accel.id.to_string(),
        name: accel.name.clone(),
        accelerator_type: format!("{:?}", accel.accelerator_type),
        vendor: accel.vendor.clone(),
        model: accel.model.clone(),
        backend_type: accel.backend_type.clone(),
        sensors: accel.sensors.values().map(|s| sensor_to_proto(s)).collect(),
        pci_bus_id: accel.pci_info.as_ref().map(|p| p.bus_id.clone()).unwrap_or_default(),
        labels: accel.labels.inner().clone(),
    }
}

pub fn sensor_to_proto(sensor: &Sensor) -> proto::Sensor {
    proto::Sensor {
        id: sensor.id.to_string(),
        name: sensor.name.clone(),
        metric_type: format!("{:?}", sensor.metric_type),
        unit: format!("{:?}", sensor.unit),
    }
}

// ============== Metric conversions ==============

pub fn metric_to_proto(metric: &Metric) -> proto::MetricSample {
    proto::MetricSample {
        accelerator_id: metric.accelerator_id.to_string(),
        sensor_id: metric.sensor_id.to_string(),
        metric_name: format!("{:?}", metric.metric_type),
        value: metric_value_to_f64(&metric.value),
        unit: format!("{:?}", metric.unit),
        timestamp_ns: metric.timestamp.nanos,
        labels: metric.labels.inner().clone(),
    }
}

pub fn metric_value_to_f64(value: &tensor_guardian_domain::entities::MetricValue) -> f64 {
    use tensor_guardian_domain::entities::MetricValue;

    match value {
        MetricValue::Scalar(v) => *v,
        MetricValue::Integer(v) => *v as f64,
        MetricValue::Memory(m) => m.used_bytes as f64,
        MetricValue::Utilization(u) => u.percent,
        MetricValue::Temperature(t) => t.celsius,
        MetricValue::Power(p) => p.watts,
        MetricValue::Frequency(f) => f.hertz as f64,
        MetricValue::Raw(_) => 0.0,
    }
}

// ============== Sample conversions ==============

pub fn sample_to_proto_vec(sample: &Sample) -> Vec<proto::MetricSample> {
    sample.metrics.iter().map(|m| metric_to_proto(m)).collect()
}

// ============== Backend health conversions ==============

use tensor_guardian_domain::ports::{BackendHealth, BackendVersion};

pub fn backend_health_to_proto(
    backend_type: &str,
    health: BackendHealth,
    version: BackendVersion,
    accelerator_count: i32,
) -> proto::BackendStatus {
    let (healthy, status_message) = match health {
        BackendHealth::Healthy => (true, "OK".to_string()),
        BackendHealth::Degraded { reason } => (true, format!("Degraded: {}", reason)),
        BackendHealth::Unhealthy { reason } => (false, format!("Unhealthy: {}", reason)),
    };

    proto::BackendStatus {
        backend_type: backend_type.to_string(),
        healthy,
        version: version.to_string(),
        status_message,
        accelerator_count,
    }
}

// ============== String ID conversions ==============

pub fn parse_accelerator_id(id: &str) -> Result<AcceleratorId, String> {
    id.parse::<uuid::Uuid>()
        .map(AcceleratorId)
        .map_err(|e| format!("Invalid accelerator ID: {}", e))
}

pub fn parse_sensor_id(id: &str) -> Result<SensorId, String> {
    id.parse::<uuid::Uuid>()
        .map(SensorId)
        .map_err(|e| format!("Invalid sensor ID: {}", e))
}

// ============== AcceleratorType parsing ==============

pub fn parse_accelerator_type(type_str: &str) -> Option<AcceleratorType> {
    match type_str {
        "NvidiaGpu" => Some(AcceleratorType::NvidiaGpu),
        "AppleNeuralEngine" => Some(AcceleratorType::AppleNeuralEngine),
        "GoogleTpu" => Some(AcceleratorType::GoogleTpu),
        "AmdGpu" => Some(AcceleratorType::AmdGpu),
        "IntelNpu" => Some(AcceleratorType::IntelNpu),
        "Cpu" => Some(AcceleratorType::Cpu),
        _ => None,
    }
}

pub fn parse_metric_type(type_str: &str) -> Option<MetricType> {
    match type_str {
        "Utilization" => Some(MetricType::Utilization),
        "MemoryUsed" => Some(MetricType::MemoryUsed),
        "MemoryTotal" => Some(MetricType::MemoryTotal),
        "Temperature" => Some(MetricType::Temperature),
        "Power" => Some(MetricType::Power),
        "Frequency" => Some(MetricType::Frequency),
        _ => None,
    }
}

pub fn parse_unit(unit_str: &str) -> Option<Unit> {
    match unit_str {
        "Percent" => Some(Unit::Percent),
        "Bytes" => Some(Unit::Bytes),
        "Celsius" => Some(Unit::Celsius),
        "Watts" => Some(Unit::Watts),
        "Megahertz" => Some(Unit::Megahertz),
        "Gigahertz" => Some(Unit::Gigahertz),
        "Microseconds" => Some(Unit::Microseconds),
        "Kilobytes" => Some(Unit::Kilobytes),
        "Megabytes" => Some(Unit::Megabytes),
        "Gigabytes" => Some(Unit::Gigabytes),
        _ => None,
    }
}
