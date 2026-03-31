//! Google Cloud TPU adapter
//!
//! Supports both local libtpu and Cloud TPU Runtime gRPC access.

use async_trait::async_trait;
use tensor_guardian_domain::{
    aggregates::{Accelerator, Sample, PciInfo},
    entities::{Metric, MetricValue, Sensor},
    ports::{AcceleratorBackend, BackendHealth, BackendVersion},
    value_objects::{
        AcceleratorId, AcceleratorType, Labels, MemoryStats, MetricType,
        Power, SensorId, Temperature, Timestamp, Unit, Utilization,
    },
    DomainResult, DomainError,
};
use tracing::{debug, info, warn};

pub mod local;
pub mod cloud;

use local::{LibTpu, TpuMetrics};
use cloud::{CloudTpuClient, detect_cloud_tpu, TpuHealth};

/// TPU backend implementation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TpuBackendType {
    /// Local libtpu
    Local,
    /// Cloud TPU Runtime
    Cloud,
}

/// Unified TPU backend
#[derive(Debug)]
pub struct TpuBackend {
    backend_type: TpuBackendType,
    local: Option<LibTpu>,
    cloud: Option<CloudTpuClient>,
}

impl TpuBackend {
    /// Create new TPU backend with auto-detection
    pub async fn new() -> DomainResult<Self> {
        // Try local libtpu first
        if let Some(libtpu) = LibTpu::try_load() {
            if libtpu.chip_count() > 0 {
                info!("Using local libtpu backend with {} chip(s)", libtpu.chip_count());
                return Ok(Self {
                    backend_type: TpuBackendType::Local,
                    local: Some(libtpu),
                    cloud: None,
                });
            }
        }
        
        // Try Cloud TPU
        if let Some(worker_addr) = detect_cloud_tpu() {
            let mut client = CloudTpuClient::new(worker_addr);
            match client.connect().await {
                Ok(_) => {
                    info!("Using Cloud TPU Runtime backend");
                    return Ok(Self {
                        backend_type: TpuBackendType::Cloud,
                        local: None,
                        cloud: Some(client),
                    });
                }
                Err(e) => {
                    warn!("Failed to connect to Cloud TPU: {}", e);
                }
            }
        }
        
        Err(DomainError::BackendError(
            "No TPU backend available (libtpu or Cloud TPU)".to_string()
        ))
    }
    
    /// Create backend with explicit type
    pub async fn with_type(backend_type: TpuBackendType) -> DomainResult<Self> {
        match backend_type {
            TpuBackendType::Local => {
                match LibTpu::try_load() {
                    Some(libtpu) if libtpu.chip_count() > 0 => Ok(Self {
                        backend_type,
                        local: Some(libtpu),
                        cloud: None,
                    }),
                    _ => Err(DomainError::BackendError(
                        "libtpu not available".to_string()
                    )),
                }
            }
            TpuBackendType::Cloud => {
                match detect_cloud_tpu() {
                    Some(addr) => {
                        let mut client = CloudTpuClient::new(addr);
                        client.connect().await
                            .map_err(|e| DomainError::BackendError(
                                format!("Failed to connect to Cloud TPU: {}", e)
                            ))?;
                        Ok(Self {
                            backend_type,
                            local: None,
                            cloud: Some(client),
                        })
                    }
                    None => Err(DomainError::BackendError(
                        "Cloud TPU not detected".to_string()
                    )),
                }
            }
        }
    }
    
    /// Check if TPU is available on this system
    pub fn is_available() -> bool {
        local::is_libtpu_available() || detect_cloud_tpu().is_some()
    }
    
    /// Get backend type
    pub fn backend_type(&self) -> TpuBackendType {
        self.backend_type
    }
    
    /// Create sensors for a TPU accelerator
    fn create_tpu_sensors(&self, accel_id: AcceleratorId, chip_index: i32) -> Vec<Sensor> {
        let mut sensors = Vec::new();
        
        // HBM memory
        sensors.push(
            Sensor::new(
                accel_id,
                &format!("hbm_memory_{}", chip_index),
                MetricType::MemoryUsed,
                Unit::Bytes,
            )
            .with_labels(Labels::new()
                .with("source", "tpu")
                .with("chip", &chip_index.to_string())),
        );
        
        // MXU utilization
        sensors.push(
            Sensor::new(
                accel_id,
                &format!("mxu_utilization_{}", chip_index),
                MetricType::Utilization,
                Unit::Percent,
            )
            .with_labels(Labels::new()
                .with("source", "tpu")
                .with("chip", &chip_index.to_string())
                .with("type", "mxu")),
        );
        
        // Temperature
        sensors.push(
            Sensor::new(
                accel_id,
                &format!("temperature_{}", chip_index),
                MetricType::Temperature,
                Unit::Celsius,
            )
            .with_labels(Labels::new()
                .with("source", "tpu")
                .with("chip", &chip_index.to_string())),
        );
        
        // Power
        sensors.push(
            Sensor::new(
                accel_id,
                &format!("power_{}", chip_index),
                MetricType::Power,
                Unit::Watts,
            )
            .with_labels(Labels::new()
                .with("source", "tpu")
                .with("chip", &chip_index.to_string())),
        );
        
        sensors
    }
}

#[async_trait]
impl AcceleratorBackend for TpuBackend {
    fn backend_type(&self) -> &'static str {
        match self.backend_type {
            TpuBackendType::Local => "tpu-local",
            TpuBackendType::Cloud => "tpu-cloud",
        }
    }
    
    fn supported_types(&self) -> Vec<AcceleratorType> {
        vec![AcceleratorType::GoogleTpu]
    }
    
    async fn discover(&self) -> DomainResult<Vec<Accelerator>> {
        let mut accelerators = Vec::new();
        
        match self.backend_type {
            TpuBackendType::Local => {
                if let Some(libtpu) = &self.local {
                    let chip_count = libtpu.chip_count();
                    
                    for i in 0..chip_count {
                        let chip_info = libtpu.get_chip_info(i)
                            .map_err(|e| DomainError::BackendError(
                                format!("Failed to get chip {} info: {}", i, e)
                            ))?;
                        
                        let mut accel = Accelerator::new(
                            &format!("TPU Chip {}", i),
                            AcceleratorType::GoogleTpu,
                            "tpu-local",
                            "Google",
                            &format!("TPU-v4-{}", i),
                        );
                        
                        // Add PCI info (TPUs are on PCIe)
                        accel = accel.with_pci_info(PciInfo {
                            bus_id: format!("tpu-{}", i),
                            domain: 0,
                            bus: 0,
                            device: i as u32,
                            function: 0,
                        });
                        
                        // Add sensors
                        let sensors = self.create_tpu_sensors(accel.id, i);
                        for sensor in sensors {
                            let _ = accel.add_sensor(sensor);
                        }
                        
                        accelerators.push(accel);
                    }
                    
                    info!("Discovered {} local TPU chip(s)", chip_count);
                }
            }
            
            TpuBackendType::Cloud => {
                if let Some(client) = &self.cloud {
                    if let Some(pod_info) = client.get_pod_info().await {
                        for i in 0..pod_info.num_chips {
                            let mut accel = Accelerator::new(
                                &format!("TPU {}-{}", pod_info.accelerator_type, i),
                                AcceleratorType::GoogleTpu,
                                "tpu-cloud",
                                "Google",
                                &pod_info.accelerator_type,
                            );
                            
                            accel = accel.with_pci_info(PciInfo {
                                bus_id: format!("cloud-tpu-{}", i),
                                domain: 0,
                                bus: 0,
                                device: i as u32,
                                function: 0,
                            });
                            
                            let sensors = self.create_tpu_sensors(accel.id, i as i32);
                            for sensor in sensors {
                                let _ = accel.add_sensor(sensor);
                            }
                            
                            accelerators.push(accel);
                        }
                        
                        info!("Discovered {} Cloud TPU chip(s)", pod_info.num_chips);
                    }
                }
            }
        }
        
        Ok(accelerators)
    }
    
    async fn collect(&self, accelerator: &Accelerator) -> DomainResult<Sample> {
        let mut metrics = Vec::new();
        
        // Parse chip index from accelerator name
        let chip_index: i32 = accelerator.name
            .split_whitespace()
            .last()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        
        match self.backend_type {
            TpuBackendType::Local => {
                if let Some(libtpu) = &self.local {
                    let tpu_metrics = libtpu.collect_metrics(chip_index)
                        .map_err(|e| DomainError::BackendError(
                            format!("Failed to collect metrics: {}", e)
                        ))?;
                    
                    // HBM memory
                    if let Some(sensor) = accelerator.sensors.values()
                        .find(|s| s.name == format!("hbm_memory_{}", chip_index)) {
                        let memory_stats = MemoryStats::new(
                            tpu_metrics.memory_used,
                            tpu_metrics.memory_total,
                        );
                        metrics.push(Metric::new(
                            sensor.id,
                            accelerator.id,
                            MetricType::MemoryUsed,
                            MetricValue::Memory(memory_stats),
                            Unit::Bytes,
                        ));
                    }
                    
                    // MXU utilization
                    if let Some(sensor) = accelerator.sensors.values()
                        .find(|s| s.name == format!("mxu_utilization_{}", chip_index)) {
                        metrics.push(Metric::new(
                            sensor.id,
                            accelerator.id,
                            MetricType::Utilization,
                            MetricValue::Utilization(Utilization::new(tpu_metrics.mxu_utilization).unwrap_or_default()),
                            Unit::Percent,
                        ));
                    }
                    
                    // Temperature
                    if let Some(sensor) = accelerator.sensors.values()
                        .find(|s| s.name == format!("temperature_{}", chip_index)) {
                        metrics.push(Metric::new(
                            sensor.id,
                            accelerator.id,
                            MetricType::Temperature,
                            MetricValue::Temperature(Temperature::new(tpu_metrics.temperature)),
                            Unit::Celsius,
                        ));
                    }
                    
                    // Power
                    if let Some(sensor) = accelerator.sensors.values()
                        .find(|s| s.name == format!("power_{}", chip_index)) {
                        metrics.push(Metric::new(
                            sensor.id,
                            accelerator.id,
                            MetricType::Power,
                            MetricValue::Power(Power::new(tpu_metrics.power)),
                            Unit::Watts,
                        ));
                    }
                }
            }
            
            TpuBackendType::Cloud => {
                if let Some(client) = &self.cloud {
                    let cloud_metrics = client.collect_metrics().await;
                    
                    if let Some(m) = cloud_metrics.get(chip_index as usize) {
                        // HBM memory
                        if let Some(sensor) = accelerator.sensors.values()
                            .find(|s| s.name == format!("hbm_memory_{}", chip_index)) {
                            let memory_bytes = (m.hbm_memory_used_gb * 1024.0 * 1024.0 * 1024.0) as u64;
                            let total_bytes = (m.hbm_memory_total_gb * 1024.0 * 1024.0 * 1024.0) as u64;
                            let memory_stats = MemoryStats::new(memory_bytes, total_bytes);
                            metrics.push(Metric::new(
                                sensor.id,
                                accelerator.id,
                                MetricType::MemoryUsed,
                                MetricValue::Memory(memory_stats),
                                Unit::Bytes,
                            ));
                        }
                        
                        // MXU utilization (duty cycle)
                        if let Some(sensor) = accelerator.sensors.values()
                            .find(|s| s.name == format!("mxu_utilization_{}", chip_index)) {
                            metrics.push(Metric::new(
                                sensor.id,
                                accelerator.id,
                                MetricType::Utilization,
                                MetricValue::Utilization(Utilization::new(m.duty_cycle_percent).unwrap_or_default()),
                                Unit::Percent,
                            ));
                        }
                        
                        // Temperature
                        if let Some(sensor) = accelerator.sensors.values()
                            .find(|s| s.name == format!("temperature_{}", chip_index)) {
                            metrics.push(Metric::new(
                                sensor.id,
                                accelerator.id,
                                MetricType::Temperature,
                                MetricValue::Temperature(Temperature::new(m.temperature_celsius)),
                                Unit::Celsius,
                            ));
                        }
                    }
                }
            }
        }
        
        Ok(Sample::new(accelerator.id, metrics))
    }
    
    async fn health(&self) -> BackendHealth {
        match self.backend_type {
            TpuBackendType::Local => {
                if let Some(libtpu) = &self.local {
                    if libtpu.chip_count() > 0 {
                        BackendHealth::Healthy
                    } else {
                        BackendHealth::Unhealthy {
                            reason: "No TPU chips detected"
                        }
                    }
                } else {
                    BackendHealth::Unhealthy {
                        reason: "libtpu not loaded"
                    }
                }
            }
            
            TpuBackendType::Cloud => {
                if let Some(client) = &self.cloud {
                    match client.health_check().await {
                        TpuHealth::Healthy => BackendHealth::Healthy,
                        TpuHealth::Degraded(reason) => BackendHealth::Degraded { reason: Box::leak(reason.into_boxed_str()) },
                        TpuHealth::Unhealthy(reason) => BackendHealth::Unhealthy { reason: Box::leak(reason.into_boxed_str()) },
                    }
                } else {
                    BackendHealth::Unhealthy {
                        reason: "Not connected to Cloud TPU"
                    }
                }
            }
        }
    }
    
    fn version(&self) -> BackendVersion {
        BackendVersion {
            major: 0,
            minor: 1,
            patch: 0,
            git_hash: None,
            build_date: None,
        }
    }
}

/// Factory for creating TPU backend
pub struct TpuBackendFactory;

impl TpuBackendFactory {
    /// Try to create TPU backend if available
    pub async fn try_create() -> Option<TpuBackend> {
        if TpuBackend::is_available() {
            match TpuBackend::new().await {
                Ok(backend) => Some(backend),
                Err(_) => None,
            }
        } else {
            None
        }
    }
    
    /// Create with explicit type
    pub async fn try_create_with_type(backend_type: TpuBackendType) -> Option<TpuBackend> {
        match TpuBackend::with_type(backend_type).await {
            Ok(backend) => Some(backend),
            Err(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_tpu_backend_is_available() {
        // Should not panic
        let _ = TpuBackend::is_available();
    }
    
    #[test]
    fn test_backend_type() {
        assert_eq!(TpuBackendType::Local, TpuBackendType::Local);
        assert_ne!(TpuBackendType::Local, TpuBackendType::Cloud);
    }
}
