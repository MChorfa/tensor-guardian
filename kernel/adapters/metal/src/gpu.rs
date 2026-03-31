//! Metal GPU metrics
//!
//! Provides GPU utilization and memory metrics via IOKit and Metal framework.

#[cfg(target_os = "macos")]
use libc::c_char;
use std::io;

/// GPU information and metrics
#[derive(Debug, Clone, Default)]
pub struct GpuMetrics {
    /// GPU name
    pub name: String,
    /// Whether GPU has unified memory with CPU
    pub has_unified_memory: bool,
    /// Total VRAM in bytes (or unified memory)
    pub total_memory: u64,
    /// Used VRAM in bytes
    pub used_memory: u64,
    /// GPU utilization percentage
    pub utilization: f64,
    /// GPU temperature in celsius (if available)
    pub temperature: Option<f64>,
    /// Power consumption in watts (if available)
    pub power: Option<f64>,
    /// Number of GPU cores
    pub core_count: u32,
}

/// Metal GPU backend
#[derive(Debug)]
pub struct MetalGpu;

impl MetalGpu {
    /// Create new Metal GPU backend
    pub fn new() -> Self {
        Self
    }

    /// Detect available GPUs
    #[cfg(target_os = "macos")]
    pub fn detect_gpus() -> io::Result<Vec<GpuInfo>> {
        use std::ffi::CString;
        
        let mut gpus = Vec::new();
        
        // Get GPU count via sysctl
        let mut gpu_count: i32 = 0;
        let mut size = std::mem::size_of::<i32>();
        
        let name = CString::new("hw.gpu.count").map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidData, e)
        })?;
        
        let result = unsafe {
            libc::sysctlbyname(
                name.as_ptr(),
                &mut gpu_count as *mut _ as *mut libc::c_void,
                &mut size,
                std::ptr::null_mut(),
                0,
            )
        };
        
        if result != 0 || gpu_count == 0 {
            // Fallback: assume at least one GPU on Apple Silicon
            if super::sysctl::is_apple_silicon() {
                gpus.push(GpuInfo {
                    name: "Apple GPU".to_string(),
                    is_apple_silicon: true,
                    pci_slot: 0,
                });
            }
        } else {
            for i in 0..gpu_count {
                gpus.push(GpuInfo {
                    name: format!("GPU {}", i),
                    is_apple_silicon: super::sysctl::is_apple_silicon(),
                    pci_slot: i as u32,
                });
            }
        }
        
        Ok(gpus)
    }

    /// Non-macOS fallback
    #[cfg(not(target_os = "macos"))]
    pub fn detect_gpus() -> io::Result<Vec<GpuInfo>> {
        Ok(Vec::new())
    }

    /// Collect metrics for a GPU
    pub fn collect_metrics(&self, gpu: &GpuInfo) -> io::Result<GpuMetrics> {
        let mut metrics = GpuMetrics::default();
        
        metrics.name = gpu.name.clone();
        metrics.has_unified_memory = gpu.is_apple_silicon;
        
        // On Apple Silicon, GPU shares memory with CPU
        if gpu.is_apple_silicon {
            let sys_metrics = super::sysctl::SystemMetrics::collect()?;
            metrics.total_memory = sys_metrics.total_memory;
            metrics.used_memory = sys_metrics.used_memory;
        }
        
        // Try to get core count
        metrics.core_count = self.get_gpu_core_count(gpu);
        
        Ok(metrics)
    }

    /// Get GPU core count
    #[cfg(target_os = "macos")]
    fn get_gpu_core_count(&self, gpu: &GpuInfo) -> u32 {
        if gpu.is_apple_silicon {
            // Try to get from sysctl
            let mut cores: i32 = 0;
            let mut size = std::mem::size_of::<i32>();
            
            if let Ok(name) = std::ffi::CString::new("hw.gpu.core_count") {
                let result = unsafe {
                    libc::sysctlbyname(
                        name.as_ptr(),
                        &mut cores as *mut _ as *mut libc::c_void,
                        &mut size,
                        std::ptr::null_mut(),
                        0,
                    )
                };
                
                if result == 0 && cores > 0 {
                    return cores as u32;
                }
            }
            
            // Fallback: estimate based on chip generation
            if let Ok(name) = std::ffi::CString::new("hw.machine") {
                let mut machine: [c_char; 64] = [0; 64];
                let mut size = machine.len();
                
                let result = unsafe {
                    libc::sysctlbyname(
                        name.as_ptr(),
                        machine.as_mut_ptr() as *mut libc::c_void,
                        &mut size,
                        std::ptr::null_mut(),
                        0,
                    )
                };
                
                if result == 0 {
                    let machine_str = unsafe {
                        std::ffi::CStr::from_ptr(machine.as_ptr())
                            .to_string_lossy()
                    };
                    
                    // Map machine type to core count (approximate)
                    return estimate_gpu_cores(&machine_str);
                }
            }
        }
        
        0
    }

    #[cfg(not(target_os = "macos"))]
    fn get_gpu_core_count(&self, _gpu: &GpuInfo) -> u32 {
        0
    }
}

impl Default for MetalGpu {
    fn default() -> Self {
        Self::new()
    }
}

/// GPU hardware information
#[derive(Debug, Clone)]
pub struct GpuInfo {
    pub name: String,
    pub is_apple_silicon: bool,
    pub pci_slot: u32,
}

/// Estimate GPU cores based on machine type
#[cfg(target_os = "macos")]
fn estimate_gpu_cores(machine: &str) -> u32 {
    // Apple Silicon GPU core counts (approximate)
    match machine {
        // M1 series
        "MacBookAir10,1" | "MacBookAir10,2" => 7,  // M1 Air
        "MacBookPro17,1" => 8,  // M1 Pro 13"
        "MacBookPro18,1" | "MacBookPro18,2" => 16, // M1 Pro/Max
        "MacBookPro18,3" | "MacBookPro18,4" => 16, // M1 Pro/Max
        "Macmini9,1" => 8,  // M1 Mini
        "iMac21,1" | "iMac21,2" => 8,  // M1 iMac
        "Mac13,1" | "Mac13,2" => 32, // M1 Ultra (Studio)
        
        // M2 series
        "Mac14,2" => 10, // M2 Air
        "Mac14,7" => 10, // M2 13" Pro
        "Mac14,5" | "Mac14,6" => 19, // M2 Pro/Max
        "Mac14,8" | "Mac14,9" => 38, // M2 Max/Ultra
        
        // M3 series
        "Mac15,3" | "Mac15,4" => 10, // M3 Air/Pro
        "Mac15,5" | "Mac15,6" => 18, // M3 Pro
        "Mac15,7" | "Mac15,8" => 40, // M3 Max
        "Mac15,9" | "Mac15,10" | "Mac15,11" => 40, // M3 Max variants
        
        _ => 8, // Default fallback
    }
}

#[cfg(not(target_os = "macos"))]
fn estimate_gpu_cores(_machine: &str) -> u32 {
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_metrics_default() {
        let metrics = GpuMetrics::default();
        assert!(metrics.name.is_empty());
        assert!(!metrics.has_unified_memory);
        assert_eq!(metrics.total_memory, 0);
    }

    #[test]
    fn test_estimate_gpu_cores() {
        let cores = estimate_gpu_cores("MacBookAir10,1");
        assert_eq!(cores, 7);
        
        let cores = estimate_gpu_cores("MacBookPro18,1");
        assert_eq!(cores, 16);
        
        let cores = estimate_gpu_cores("Unknown");
        assert_eq!(cores, 8); // Default
    }

    #[test]
    fn test_detect_gpus() {
        let result = MetalGpu::detect_gpus();
        // Should not panic on any platform
        let _ = result;
    }
}
