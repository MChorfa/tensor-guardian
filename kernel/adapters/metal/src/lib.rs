//! Metal/Neural Engine adapter for Apple platforms
//!
//! Provides monitoring for Apple Silicon GPUs and Neural Engine.

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
use tracing::{info, warn};

pub mod sysctl;
pub mod gpu;
pub mod ane;

use sysctl::SystemMetrics;
use gpu::MetalGpu;
use ane::AneBackend;

/// Metal backend for Apple GPUs and Neural Engine
#[derive(Debug)]
pub struct MetalBackend {
    gpu: MetalGpu,
    ane: AneBackend,
}

impl MetalBackend {
    /// Create new Metal backend
    pub fn new() -> DomainResult<Self> {
        #[cfg(not(target_os = "macos"))]
        {
            return Err(DomainError::BackendError(
                "Metal backend only available on macOS".to_string()
            ));
        }
        
        #[cfg(target_os = "macos")]
        {
            if !sysctl::is_apple_silicon() {
                return Err(DomainError::BackendError(
                    "Metal backend requires Apple Silicon".to_string()
                ));
            }
            
            info!("Metal backend initialized for Apple Silicon");
            
            Ok(Self {
                gpu: MetalGpu::new(),
                ane: AneBackend::new(),
            })
        }
    }

    /// Check if Metal backend is available
    pub fn is_available() -> bool {
        #[cfg(not(target_os = "macos"))]
        return false;
        
        #[cfg(target_os = "macos")]
        sysctl::is_apple_silicon()
    }

    /// Create sensors for a GPU accelerator
    fn create_gpu_sensors(&self, accel_id: AcceleratorId) -> Vec<Sensor> {
        let mut sensors = Vec::new();
        
        // GPU utilization
        sensors.push(
            Sensor::new(
                accel_id,
                "gpu_utilization",
                MetricType::Utilization,
                Unit::Percent,
            )
            .with_labels(Labels::new().with("source", "metal").with("type", "gpu")),
        );
        
        // Memory used (unified memory on Apple Silicon)
        sensors.push(
            Sensor::new(
                accel_id,
                "memory_used",
                MetricType::MemoryUsed,
                Unit::Bytes,
            )
            .with_labels(Labels::new().with("source", "metal").with("type", "unified")),
        );
        
        // Temperature (if available)
        sensors.push(
            Sensor::new(
                accel_id,
                "gpu_temperature",
                MetricType::Temperature,
                Unit::Celsius,
            )
            .with_labels(Labels::new().with("source", "metal")),
        );
        
        // Power consumption
        sensors.push(
            Sensor::new(
                accel_id,
                "power_draw",
                MetricType::Power,
                Unit::Watts,
            )
            .with_labels(Labels::new().with("source", "metal")),
        );
        
        sensors
    }

    /// Create sensors for ANE accelerator
    fn create_ane_sensors(&self, accel_id: AcceleratorId) -> Vec<Sensor> {
        let mut sensors = Vec::new();
        
        // ANE power
        sensors.push(
            Sensor::new(
                accel_id,
                "ane_power",
                MetricType::Power,
                Unit::Watts,
            )
            .with_labels(Labels::new().with("source", "ane")),
        );
        
        // ANE utilization (estimated)
        sensors.push(
            Sensor::new(
                accel_id,
                "ane_utilization",
                MetricType::Utilization,
                Unit::Percent,
            )
            .with_labels(Labels::new().with("source", "ane")),
        );
        
        // ANE temperature
        sensors.push(
            Sensor::new(
                accel_id,
                "ane_temperature",
                MetricType::Temperature,
                Unit::Celsius,
            )
            .with_labels(Labels::new().with("source", "ane")),
        );
        
        sensors
    }
}

#[async_trait]
impl AcceleratorBackend for MetalBackend {
    fn backend_type(&self) -> &'static str {
        "metal"
    }

    fn supported_types(&self) -> Vec<AcceleratorType> {
        vec![
            AcceleratorType::AppleGpu,
            AcceleratorType::AppleNeuralEngine,
        ]
    }

    async fn discover(&self) -> DomainResult<Vec<Accelerator>> {
        let mut accelerators = Vec::new();
        
        // Discover GPUs
        match MetalGpu::detect_gpus() {
            Ok(gpus) => {
                for gpu_info in gpus {
                    let mut accel = Accelerator::new(
                        &gpu_info.name,
                        AcceleratorType::AppleGpu,
                        "metal",
                        "Apple",
                        &gpu_info.name,
                    );
                    
                    // Add PCI info (placeholder for Apple Silicon)
                    accel = accel.with_pci_info(PciInfo {
                        bus_id: format!("internal-{}", gpu_info.pci_slot),
                        domain: 0,
                        bus: 0,
                        device: gpu_info.pci_slot as u16,
                        function: 0,
                    });
                    
                    // Add GPU sensors
                    let sensors = self.create_gpu_sensors(accel.id);
                    for sensor in sensors {
                        let _ = accel.add_sensor(sensor);
                    }
                    
                    accelerators.push(accel);
                }
                
                info!("Discovered {} Apple GPU(s)", accelerators.len());
            }
            Err(e) => {
                warn!("Failed to detect GPUs: {}", e);
            }
        }
        
        // Discover Neural Engine (if available)
        if AneBackend::is_available() {
            let ane_gen = ane::detect_ane_generation();
            let ane_name = format!("Apple Neural Engine ({:?})", ane_gen);
            
            let mut ane_accel = Accelerator::new(
                &ane_name,
                AcceleratorType::AppleNeuralEngine,
                "metal",
                "Apple",
                &ane_name,
            );
            
            // Add ANE sensors
            let sensors = self.create_ane_sensors(ane_accel.id);
            for sensor in sensors {
                let _ = ane_accel.add_sensor(sensor);
            }
            
            accelerators.push(ane_accel);
            info!("Discovered Apple Neural Engine ({:?})", ane_gen);
        }
        
        Ok(accelerators)
    }

    async fn collect(&self, accelerator: &Accelerator) -> DomainResult<Sample> {
        let mut metrics = Vec::new();
        
        match accelerator.accelerator_type {
            AcceleratorType::AppleGpu => {
                // Collect system metrics (includes GPU via unified memory)
                let sys_metrics = SystemMetrics::collect()
                    .map_err(|e| DomainError::BackendError(format!("Failed to collect: {}", e)))?;
                
                // GPU utilization (estimate from thermal state)
                let utilization = match sys_metrics.thermal_state {
                    0 => 30.0, // Normal - assume moderate load
                    1 => 60.0, // Fair - higher load
                    2 => 80.0, // Serious - heavy load
                    3 => 95.0, // Critical - max load
                    _ => 30.0,
                };
                
                if let Some(sensor) = accelerator.sensors.values()
                    .find(|s| s.name == "gpu_utilization") {
                    metrics.push(Metric::new(
                        sensor.id,
                        accelerator.id,
                        MetricType::Utilization,
                        MetricValue::Utilization(Utilization::new(utilization).unwrap_or_default()),
                        Unit::Percent,
                    ));
                }
                
                // Memory used (unified memory)
                if let Some(sensor) = accelerator.sensors.values()
                    .find(|s| s.metric_type == MetricType::MemoryUsed) {
                    let memory_stats = MemoryStats::new(
                        sys_metrics.used_memory,
                        sys_metrics.total_memory,
                    );
                    metrics.push(Metric::new(
                        sensor.id,
                        accelerator.id,
                        MetricType::MemoryUsed,
                        MetricValue::Memory(memory_stats),
                        Unit::Bytes,
                    ));
                }
                
                // Temperature (from thermal state)
                let temp = match sys_metrics.thermal_state {
                    0 => 45.0,
                    1 => 60.0,
                    2 => 75.0,
                    3 => 85.0,
                    _ => 50.0,
                };
                
                if let Some(sensor) = accelerator.sensors.values()
                    .find(|s| s.name == "gpu_temperature") {
                    metrics.push(Metric::new(
                        sensor.id,
                        accelerator.id,
                        MetricType::Temperature,
                        MetricValue::Temperature(Temperature::new(temp)),
                        Unit::Celsius,
                    ));
                }
                
                // Power draw (estimated)
                let power = match sys_metrics.thermal_state {
                    0 => 8.0,  // Idle-ish
                    1 => 15.0, // Moderate
                    2 => 25.0, // Heavy
                    3 => 30.0, // Max
                    _ => 10.0,
                };
                
                if let Some(sensor) = accelerator.sensors.values()
                    .find(|s| s.name == "power_draw") {
                    metrics.push(Metric::new(
                        sensor.id,
                        accelerator.id,
                        MetricType::Power,
                        MetricValue::Power(Power::new(power)),
                        Unit::Watts,
                    ));
                }
            }
            
            AcceleratorType::AppleNeuralEngine => {
                let ane_metrics = self.ane.collect_metrics()
                    .map_err(|e| DomainError::BackendError(format!("ANE collect failed: {}", e)))?;
                
                if ane_metrics.available {
                    // ANE power
                    if let Some(sensor) = accelerator.sensors.values()
                        .find(|s| s.name == "ane_power") {
                        metrics.push(Metric::new(
                            sensor.id,
                            accelerator.id,
                            MetricType::Power,
                            MetricValue::Power(Power::new(ane_metrics.power_watts)),
                            Unit::Watts,
                        ));
                    }
                    
                    // ANE temperature
                    if let Some(sensor) = accelerator.sensors.values()
                        .find(|s| s.name == "ane_temperature") {
                        metrics.push(Metric::new(
                            sensor.id,
                            accelerator.id,
                            MetricType::Temperature,
                            MetricValue::Temperature(Temperature::new(ane_metrics.temperature_celsius)),
                            Unit::Celsius,
                        ));
                    }
                    
                    // ANE utilization (estimated from power)
                    let utilization = if ane_metrics.power_watts > 2.0 {
                        80.0
                    } else if ane_metrics.power_watts > 1.0 {
                        50.0
                    } else {
                        20.0
                    };
                    
                    if let Some(sensor) = accelerator.sensors.values()
                        .find(|s| s.name == "ane_utilization") {
                        metrics.push(Metric::new(
                            sensor.id,
                            accelerator.id,
                            MetricType::Utilization,
                            MetricValue::Utilization(Utilization::new(utilization).unwrap_or_default()),
                            Unit::Percent,
                        ));
                    }
                }
            }
            
            _ => {}
        }
        
        Ok(Sample::new(accelerator.id, metrics))
    }

    async fn health(&self) -> BackendHealth {
        #[cfg(not(target_os = "macos"))]
        {
            return BackendHealth::Unhealthy { 
                reason: "Metal not available on this platform" 
            };
        }
        
        #[cfg(target_os = "macos")]
        {
            if !Self::is_available() {
                return BackendHealth::Unhealthy {
                    reason: "Apple Silicon not detected"
                };
            }
            
            // Check thermal state
            match SystemMetrics::collect() {
                Ok(metrics) => {
                    match metrics.thermal_state {
                        0 | 1 => BackendHealth::Healthy,
                        2 => BackendHealth::Degraded { 
                            reason: "Thermal throttling active" 
                        },
                        3 => BackendHealth::Unhealthy { 
                            reason: "Critical thermal state" 
                        },
                        _ => BackendHealth::Healthy,
                    }
                }
                Err(_) => BackendHealth::Degraded {
                    reason: "Failed to query system metrics"
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

/// Factory for creating Metal backend
pub struct MetalBackendFactory;

impl MetalBackendFactory {
    /// Try to create Metal backend if available
    pub fn try_create() -> Option<MetalBackend> {
        if MetalBackend::is_available() {
            match MetalBackend::new() {
                Ok(backend) => Some(backend),
                Err(_) => None,
            }
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metal_backend_is_available() {
        // Should not panic on any platform
        let _ = MetalBackend::is_available();
    }

    #[test]
    fn test_factory_try_create() {
        // Should not panic
        let _ = MetalBackendFactory::try_create();
    }
}
