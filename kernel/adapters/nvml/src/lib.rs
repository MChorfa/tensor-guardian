use async_trait::async_trait;
use nvml_wrapper::NVML;
use nvml_wrapper::error::NvmlError;
use tensor_guardian_domain::{
    aggregates::{Accelerator, PciInfo, Sample},
    entities::{Metric, MetricValue, Sensor},
    ports::{AcceleratorBackend, BackendHealth, BackendVersion},
    value_objects::{
        AcceleratorId, AcceleratorType, Frequency, Labels, MemoryStats, 
        MetricType, Power, SensorId, Temperature, Timestamp, Unit, Utilization,
    },
    DomainError, DomainResult,
};
use std::collections::HashMap;
use tracing::{debug, error, info, warn};

/// NVML backend for NVIDIA GPU monitoring
#[derive(Debug)]
pub struct NvmlBackend {
    nvml: NVML,
}

impl NvmlBackend {
    pub fn new() -> DomainResult<Self> {
        match NVML::init() {
            Ok(nvml) => {
                info!("NVML initialized successfully");
                Ok(Self { nvml })
            }
            Err(e) => {
                error!("Failed to initialize NVML: {}", e);
                Err(DomainError::BackendError(format!("NVML init failed: {}", e)))
            }
        }
    }

    fn convert_nvml_error(e: NvmlError) -> DomainError {
        match e {
            NvmlError::NoPermission => DomainError::BackendError("NVML: No permission".to_string()),
            NvmlError::NotSupported => DomainError::BackendError("NVML: Feature not supported".to_string()),
            NvmlError::GpuLost => DomainError::BackendError("NVML: GPU lost".to_string()),
            _ => DomainError::BackendError(format!("NVML: {}", e)),
        }
    }

    fn create_sensors_for_device(&self, device: &nvml_wrapper::Device, accel_id: AcceleratorId) -> Vec<Sensor> {
        let mut sensors = Vec::new();

        // GPU Utilization sensor
        sensors.push(
            Sensor::new(
                accel_id,
                "gpu_utilization",
                MetricType::Utilization,
                Unit::Percent,
            )
            .with_labels(Labels::new().with("source", "nvml").with("type", "compute")),
        );

        // Memory utilization sensor
        sensors.push(
            Sensor::new(
                accel_id,
                "memory_utilization",
                MetricType::Utilization,
                Unit::Percent,
            )
            .with_labels(Labels::new().with("source", "nvml").with("type", "memory")),
        );

        // Temperature sensor
        sensors.push(
            Sensor::new(
                accel_id,
                "gpu_temperature",
                MetricType::Temperature,
                Unit::Celsius,
            )
            .with_labels(Labels::new().with("source", "nvml")),
        );

        // Power sensor
        sensors.push(
            Sensor::new(
                accel_id,
                "power_draw",
                MetricType::Power,
                Unit::Watts,
            )
            .with_labels(Labels::new().with("source", "nvml")),
        );

        // Clock sensors
        sensors.push(
            Sensor::new(
                accel_id,
                "graphics_clock",
                MetricType::Frequency,
                Unit::Megahertz,
            )
            .with_labels(Labels::new().with("source", "nvml").with("clock_type", "graphics")),
        );

        sensors.push(
            Sensor::new(
                accel_id,
                "memory_clock",
                MetricType::Frequency,
                Unit::Megahertz,
            )
            .with_labels(Labels::new().with("source", "nvml").with("clock_type", "memory")),
        );

        // Fan speed (may not be available on all devices)
        if device.fan_speed().is_ok() {
            sensors.push(
                Sensor::new(
                    accel_id,
                    "fan_speed",
                    MetricType::FanSpeed,
                    Unit::Percent,
                )
                .with_labels(Labels::new().with("source", "nvml")),
            );
        }

        // Encoder/Decoder utilization (if supported)
        if device.encoder_utilization().is_ok() {
            sensors.push(
                Sensor::new(
                    accel_id,
                    "encoder_utilization",
                    MetricType::Utilization,
                    Unit::Percent,
                )
                .with_labels(Labels::new().with("source", "nvml").with("type", "encoder")),
            );
        }

        if device.decoder_utilization().is_ok() {
            sensors.push(
                Sensor::new(
                    accel_id,
                    "decoder_utilization",
                    MetricType::Utilization,
                    Unit::Percent,
                )
                .with_labels(Labels::new().with("source", "nvml").with("type", "decoder")),
            );
        }

        sensors
    }
}

#[async_trait]
impl AcceleratorBackend for NvmlBackend {
    fn backend_type(&self) -> &'static str {
        "nvml"
    }

    fn supported_types(&self) -> Vec<AcceleratorType> {
        vec![AcceleratorType::NvidiaGpu]
    }

    async fn discover(&self) -> DomainResult<Vec<Accelerator>> {
        let count = self.nvml.device_count()
            .map_err(Self::convert_nvml_error)?;
        
        debug!("NVML reports {} devices", count);
        
        let mut accelerators = Vec::with_capacity(count as usize);
        
        for i in 0..count {
            let device = self.nvml.device_by_index(i)
                .map_err(Self::convert_nvml_error)?;
            
            let name = device.name()
                .map_err(Self::convert_nvml_error)?;
            
            let pci_info = device.pci_info()
                .map_err(Self::convert_nvml_error)?;
            
            let mut accel = Accelerator::new(
                &name,
                AcceleratorType::NvidiaGpu,
                "nvml",
                "NVIDIA",
                &name,
            );
            
            // Add PCI info
            accel = accel.with_pci_info(PciInfo {
                bus_id: pci_info.bus_id.clone(),
                domain: pci_info.domain,
                bus: pci_info.bus,
                device: pci_info.device,
                function: pci_info.function,
            });
            
            // Add version info
            if let Ok(driver_version) = self.nvml.sys_driver_version() {
                accel = accel.with_driver_version(driver_version);
            }
            
            if let Ok(nvml_version) = self.nvml.sys_nvml_version() {
                accel = accel.with_firmware_version(nvml_version);
            }
            
            // Add serial number
            if let Ok(serial) = device.serial() {
                accel = accel.with_serial_number(serial);
            }
            
            // Create and add sensors
            let sensors = self.create_sensors_for_device(&device, accel.id);
            for sensor in sensors {
                let _ = accel.add_sensor(sensor);
            }
            
            accelerators.push(accel);
        }
        
        info!("Discovered {} NVIDIA GPUs", accelerators.len());
        Ok(accelerators)
    }

    async fn collect(&self, accelerator: &Accelerator) -> DomainResult<Sample> {
        // Find device by PCI info
        let pci_info = accelerator.pci_info.as_ref()
            .ok_or_else(|| DomainError::BackendError("Missing PCI info".to_string()))?;
        
        let device = self.nvml.device_by_pci_id(
            pci_info.domain as u32,
            pci_info.bus as u32,
            pci_info.device as u32,
        ).map_err(Self::convert_nvml_error)?;
        
        let mut metrics = Vec::new();
        
        // Collect utilization
        match device.utilization_rates() {
            Ok(util) => {
                metrics.push(Metric::new(
                    accelerator.sensors.values()
                        .find(|s| s.name == "gpu_utilization")
                        .map(|s| s.id)
                        .unwrap_or_else(SensorId::new),
                    accelerator.id,
                    MetricType::Utilization,
                    MetricValue::Utilization(Utilization::new(util.gpu as f64).unwrap_or_default()),
                    Unit::Percent,
                ));
                
                metrics.push(Metric::new(
                    accelerator.sensors.values()
                        .find(|s| s.name == "memory_utilization")
                        .map(|s| s.id)
                        .unwrap_or_else(SensorId::new),
                    accelerator.id,
                    MetricType::Utilization,
                    MetricValue::Utilization(Utilization::new(util.memory as f64).unwrap_or_default()),
                    Unit::Percent,
                ));
            }
            Err(e) => {
                warn!("Failed to get utilization for {}: {}", accelerator.id, e);
            }
        }
        
        // Collect memory info
        match device.memory_info() {
            Ok(mem) => {
                let memory_stats = MemoryStats::new(mem.used, mem.total);
                metrics.push(Metric::new(
                    accelerator.sensors.values()
                        .find(|s| s.metric_type == MetricType::MemoryUsed)
                        .map(|s| s.id)
                        .unwrap_or_else(SensorId::new),
                    accelerator.id,
                    MetricType::MemoryUsed,
                    MetricValue::Memory(memory_stats),
                    Unit::Bytes,
                ));
            }
            Err(NvmlError::NotSupported) => {
                // Unified memory system (DGX Spark) - report as unified
                debug!("Unified memory detected for {}", accelerator.id);
            }
            Err(e) => {
                warn!("Failed to get memory info for {}: {}", accelerator.id, e);
            }
        }
        
        // Collect temperature
        match device.temperature(nvml_wrapper::enum_wrappers::device::TemperatureSensor::Gpu) {
            Ok(temp) => {
                metrics.push(Metric::new(
                    accelerator.sensors.values()
                        .find(|s| s.name == "gpu_temperature")
                        .map(|s| s.id)
                        .unwrap_or_else(SensorId::new),
                    accelerator.id,
                    MetricType::Temperature,
                    MetricValue::Temperature(Temperature::new(temp as f64)),
                    Unit::Celsius,
                ));
            }
            Err(e) => {
                debug!("Failed to get temperature for {}: {}", accelerator.id, e);
            }
        }
        
        // Collect power
        match device.power_usage() {
            Ok(power_mw) => {
                metrics.push(Metric::new(
                    accelerator.sensors.values()
                        .find(|s| s.name == "power_draw")
                        .map(|s| s.id)
                        .unwrap_or_else(SensorId::new),
                    accelerator.id,
                    MetricType::Power,
                    MetricValue::Power(Power::new(power_mw as f64 / 1000.0)),
                    Unit::Watts,
                ));
            }
            Err(e) => {
                debug!("Failed to get power for {}: {}", accelerator.id, e);
            }
        }
        
        // Collect clocks
        match device.clock_info(nvml_wrapper::enum_wrappers::device::Clock::Graphics) {
            Ok(clock_mhz) => {
                metrics.push(Metric::new(
                    accelerator.sensors.values()
                        .find(|s| s.name == "graphics_clock")
                        .map(|s| s.id)
                        .unwrap_or_else(SensorId::new),
                    accelerator.id,
                    MetricType::Frequency,
                    MetricValue::Frequency(Frequency::from_mhz(clock_mhz)),
                    Unit::Megahertz,
                ));
            }
            Err(e) => {
                debug!("Failed to get graphics clock for {}: {}", accelerator.id, e);
            }
        }
        
        match device.clock_info(nvml_wrapper::enum_wrappers::device::Clock::Memory) {
            Ok(clock_mhz) => {
                metrics.push(Metric::new(
                    accelerator.sensors.values()
                        .find(|s| s.name == "memory_clock")
                        .map(|s| s.id)
                        .unwrap_or_else(SensorId::new),
                    accelerator.id,
                    MetricType::Frequency,
                    MetricValue::Frequency(Frequency::from_mhz(clock_mhz)),
                    Unit::Megahertz,
                ));
            }
            Err(e) => {
                debug!("Failed to get memory clock for {}: {}", accelerator.id, e);
            }
        }
        
        Ok(Sample::new(accelerator.id, metrics))
    }

    async fn health(&self) -> BackendHealth {
        match self.nvml.device_count() {
            Ok(_) => BackendHealth::Healthy,
            Err(e) => BackendHealth::Unhealthy { 
                reason: Box::leak(format!("NVML error: {}", e).into_boxed_str()) 
            },
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

/// Factory for creating NVML backend
pub struct NvmlBackendFactory;

impl NvmlBackendFactory {
    pub fn try_create() -> Option<Box<dyn AcceleratorBackend>> {
        match NvmlBackend::new() {
            Ok(backend) => Some(Box::new(backend)),
            Err(_) => None,
        }
    }
}
