//! libtpu local backend for direct TPU access
//!
//! Dynamically loads libtpu.so and queries local TPU metrics.

use libloading::{Library, Symbol};
use std::io;
use tracing::{debug, error, warn};

/// libtpu library handle
#[derive(Debug)]
pub struct LibTpu {
    _lib: Library,
    chip_count: i32,
}

/// TPU chip information
#[derive(Debug, Clone)]
pub struct TpuChipInfo {
    pub index: i32,
    pub memory_capacity_gb: u64,
    pub hbm_bandwidth_gbps: f64,
}

/// TPU metrics
#[derive(Debug, Clone, Default)]
pub struct TpuMetrics {
    /// HBM memory used in bytes
    pub memory_used: u64,
    /// HBM memory total in bytes
    pub memory_total: u64,
    /// MXU utilization percentage
    pub mxu_utilization: f64,
    /// TPU temperature in celsius
    pub temperature: f64,
    /// TPU power consumption in watts
    pub power: f64,
    /// Infeed/Outfeed throughput
    pub infeed_throughput_gbps: f64,
    pub outfeed_throughput_gbps: f64,
}

impl LibTpu {
    /// Try to load libtpu
    pub fn try_load() -> Option<Self> {
        let lib_paths = [
            "/usr/lib/libtpu.so",
            "/opt/google/libtpu/libtpu.so",
            "libtpu.so",
        ];

        for path in &lib_paths {
            match Self::load_from_path(path) {
                Ok(libtpu) => {
                    debug!("Loaded libtpu from {}", path);
                    return Some(libtpu);
                }
                Err(e) => {
                    debug!("Failed to load libtpu from {}: {}", path, e);
                }
            }
        }

        warn!("libtpu not found on system");
        None
    }

    /// Load libtpu from specific path
    fn load_from_path(path: &str) -> io::Result<Self> {
        let lib =
            unsafe { Library::new(path) }.map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        // Try to get chip count
        let chip_count: i32 =
            match unsafe { lib.get::<Symbol<extern "C" fn() -> i32>>(b"TpuDriver_Count\0") } {
                Ok(count_fn) => count_fn(),
                Err(_) => {
                    // Try alternative symbol names
                    match unsafe { lib.get::<Symbol<extern "C" fn() -> i32>>(b"tpu_count\0") } {
                        Ok(count_fn) => count_fn(),
                        Err(_) => 0,
                    }
                }
            };

        if chip_count == 0 {
            warn!("No TPU chips detected via libtpu");
        } else {
            debug!("Detected {} TPU chip(s)", chip_count);
        }

        Ok(Self {
            _lib: lib,
            chip_count,
        })
    }

    /// Get number of TPU chips
    pub fn chip_count(&self) -> i32 {
        self.chip_count
    }

    /// Get chip info for a specific chip
    pub fn get_chip_info(&self, index: i32) -> io::Result<TpuChipInfo> {
        if index < 0 || index >= self.chip_count {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Invalid chip index: {} (count: {})", index, self.chip_count),
            ));
        }

        // TODO: Query actual chip info via libtpu
        // For now, return typical TPU v4 specs
        Ok(TpuChipInfo {
            index,
            memory_capacity_gb: 32, // TPU v4 has 32GB HBM per chip
            hbm_bandwidth_gbps: 1200.0,
        })
    }

    /// Collect metrics for a specific chip
    pub fn collect_metrics(&self, index: i32) -> io::Result<TpuMetrics> {
        let _chip_info = self.get_chip_info(index)?;

        // TODO: Query actual metrics via libtpu
        // This would use TpuDriver_Metrics or similar API

        // For now, return placeholder metrics
        Ok(TpuMetrics {
            memory_total: 32 * 1024 * 1024 * 1024, // 32 GB
            memory_used: 16 * 1024 * 1024 * 1024,  // 16 GB (50%)
            mxu_utilization: 65.0,
            temperature: 72.0,
            power: 175.0,
            infeed_throughput_gbps: 45.0,
            outfeed_throughput_gbps: 42.0,
        })
    }
}

/// Check if libtpu is available on this system
pub fn is_libtpu_available() -> bool {
    LibTpu::try_load().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_libtpu_available() {
        // Should not panic
        let _ = is_libtpu_available();
    }

    #[test]
    fn test_tpu_metrics_default() {
        let metrics = TpuMetrics::default();
        assert_eq!(metrics.memory_total, 0);
        assert_eq!(metrics.mxu_utilization, 0.0);
    }
}
