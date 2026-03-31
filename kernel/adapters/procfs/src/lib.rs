//! Linux procfs adapter for CPU and memory metrics
//! 
//! Reads from /proc/stat, /proc/meminfo, /proc/cpuinfo

use async_trait::async_trait;
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
use tokio::fs;

/// Procfs backend for CPU/memory monitoring
#[derive(Debug)]
pub struct ProcfsBackend;

impl ProcfsBackend {
    pub fn new() -> DomainResult<Self> {
        Ok(Self)
    }

    async fn read_cpu_info(&self) -> DomainResult<Vec<(String, u64, u64)>> {
        let content = fs::read_to_string("/proc/stat").await
            .map_err(|e| DomainError::BackendError(format!("Failed to read /proc/stat: {}", e)))?;
        
        let mut cpus = Vec::new();
        for line in content.lines() {
            if line.starts_with("cpu") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 5 {
                    let name = parts[0].to_string();
                    let user = parts[1].parse::<u64>().unwrap_or(0);
                    let system = parts[3].parse::<u64>().unwrap_or(0);
                    let idle = parts[4].parse::<u64>().unwrap_or(0);
                    cpus.push((name, user + system, idle));
                }
            }
        }
        Ok(cpus)
    }

    async fn read_mem_info(&self) -> DomainResult<MemoryStats> {
        let content = fs::read_to_string("/proc/meminfo").await
            .map_err(|e| DomainError::BackendError(format!("Failed to read /proc/meminfo: {}", e)))?;
        
        let mut total = 0u64;
        let mut free = 0u64;
        let mut buffers = 0u64;
        let mut cached = 0u64;
        
        for line in content.lines() {
            if line.starts_with("MemTotal:") {
                total = parse_kb(line)?;
            } else if line.starts_with("MemFree:") {
                free = parse_kb(line)?;
            } else if line.starts_with("Buffers:") {
                buffers = parse_kb(line)?;
            } else if line.starts_with("Cached:") {
                cached = parse_kb(line)?;
            }
        }
        
        let used = total - free - buffers - cached;
        Ok(MemoryStats::new(used * 1024, total * 1024))
    }
}

fn parse_kb(line: &str) -> DomainResult<u64> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() >= 2 {
        parts[1].parse::<u64>()
            .map_err(|e| DomainError::BackendError(format!("Parse error: {}", e)))
    } else {
        Err(DomainError::BackendError("Invalid format".to_string()))
    }
}

#[async_trait]
impl AcceleratorBackend for ProcfsBackend {
    fn backend_type(&self) -> &'static str {
        "procfs"
    }

    fn supported_types(&self) -> Vec<AcceleratorType> {
        vec![AcceleratorType::Cpu]
    }

    async fn discover(&self) -> DomainResult<Vec<Accelerator>> {
        // Create a CPU accelerator
        let mut accel = Accelerator::new(
            "cpu",
            AcceleratorType::Cpu,
            "procfs",
            "Linux",
            "Generic CPU",
        );

        // Add CPU sensors
        let cpu_util_sensor = Sensor::new(
            accel.id,
            "cpu_utilization",
            MetricType::Utilization,
            Unit::Percent,
        );
        let _ = accel.add_sensor(cpu_util_sensor);

        let mem_sensor = Sensor::new(
            accel.id,
            "memory_usage",
            MetricType::MemoryUsed,
            Unit::Bytes,
        );
        let _ = accel.add_sensor(mem_sensor);

        Ok(vec![accel])
    }

    async fn collect(&self, accelerator: &Accelerator) -> DomainResult<Sample> {
        let mut metrics = Vec::new();

        // Collect memory info
        let mem_stats = self.read_mem_info().await?;
        metrics.push(Metric::new(
            accelerator.sensors.values()
                .find(|s| s.name == "memory_usage")
                .map(|s| s.id)
                .unwrap_or_else(SensorId::new),
            accelerator.id,
            MetricType::MemoryUsed,
            MetricValue::Memory(mem_stats),
            Unit::Bytes,
        ));

        // Collect CPU info
        let cpus = self.read_cpu_info().await?;
        if let Some((_, active, _)) = cpus.first() {
            // Calculate rough utilization (this is simplified - should use deltas)
            let utilization = (*active % 100) as f64;
            metrics.push(Metric::new(
                accelerator.sensors.values()
                    .find(|s| s.name == "cpu_utilization")
                    .map(|s| s.id)
                    .unwrap_or_else(SensorId::new),
                accelerator.id,
                MetricType::Utilization,
                MetricValue::Utilization(Utilization::new(utilization.min(100.0)).unwrap_or_default()),
                Unit::Percent,
            ));
        }

        Ok(Sample::new(accelerator.id, metrics))
    }

    async fn health(&self) -> BackendHealth {
        // Check if we can read /proc
        match tokio::fs::read_to_string("/proc/stat").await {
            Ok(_) => BackendHealth::Healthy,
            Err(_) => BackendHealth::Unhealthy { reason: "Cannot read /proc" },
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
