//! macOS sysctl/mach system metrics
//!
//! Provides CPU, memory, and thermal metrics via Mach kernel APIs.

#[cfg(target_os = "macos")]
use mach2::{
    kern_return::kern_return_t,
    mach_init::mach_host_self,
    mach_types::{host_t, processor_flavor_t},
    message::mach_msg_type_number_t,
    port::MACH_PORT_NULL,
    processor::{
        processor_cpu_load_info, processor_cpu_load_info_t, processor_flavor_t as proc_flavor,
    },
    traps::mach_task_self,
    vm_statistics::{vm_statistics64, vm_statistics64_data_t, VM_STATISTICS64_COUNT},
};
#[cfg(target_os = "macos")]
use libc::{c_int, c_void, size_t};
use std::io;

/// System metrics for macOS
#[derive(Debug, Clone, Default)]
pub struct SystemMetrics {
    /// CPU utilization percentage (0-100)
    pub cpu_percent: f64,
    /// Memory pressure level (0-4, where 4 is critical)
    pub memory_pressure: u32,
    /// Thermal state (0-3, where 3 is critical)
    pub thermal_state: u32,
    /// Total system memory in bytes
    pub total_memory: u64,
    /// Free memory in bytes
    pub free_memory: u64,
    /// Used memory in bytes
    pub used_memory: u64,
}

impl SystemMetrics {
    /// Collect current system metrics
    #[cfg(target_os = "macos")]
    pub fn collect() -> io::Result<Self> {
        let mut metrics = Self::default();
        
        // Get CPU load info
        metrics.cpu_percent = Self::get_cpu_usage()?;
        
        // Get VM statistics
        let vm_stats = Self::get_vm_statistics()?;
        metrics.total_memory = vm_stats.total_memory;
        metrics.free_memory = vm_stats.free_memory;
        metrics.used_memory = vm_stats.used_memory;
        
        // Get thermal state
        metrics.thermal_state = Self::get_thermal_state().unwrap_or(0);
        
        // Get memory pressure
        metrics.memory_pressure = Self::get_memory_pressure().unwrap_or(0);
        
        Ok(metrics)
    }
    
    /// Non-macOS fallback returns zeros
    #[cfg(not(target_os = "macos"))]
    pub fn collect() -> io::Result<Self> {
        Ok(Self::default())
    }
    
    /// Get CPU usage percentage via processor_info
    #[cfg(target_os = "macos")]
    fn get_cpu_usage() -> io::Result<f64> {
        unsafe {
            let host = mach_host_self();
            let mut processor_count: mach_msg_type_number_t = 0;
            let mut cpu_load_info: processor_cpu_load_info_t = std::ptr::null_mut();
            let mut info_count: mach_msg_type_number_t = 0;
            
            let result = mach2::processor::processor_info(
                host,
                proc_flavor::PROCESSOR_CPU_LOAD_INFO as processor_flavor_t,
                &mut processor_count,
                &mut cpu_load_info as *mut _ as *mut c_int,
                &mut info_count,
            );
            
            if result != 0 {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("processor_info failed: {}", result),
                ));
            }
            
            // Calculate CPU usage from load info
            // cpu_load_info contains ticks for user, system, idle
            // We need two samples to calculate percentage
            // For simplicity, return 0.0 (full implementation would track history)
            
            // Free the info buffer
            let _ = mach2::vm::vm_deallocate(
                mach_task_self(),
                cpu_load_info as usize,
                info_count as usize * std::mem::size_of::<processor_cpu_load_info>(),
            );
            
            Ok(0.0) // Placeholder - real implementation needs sampling
        }
    }
    
    /// Get VM statistics via host_statistics64
    #[cfg(target_os = "macos")]
    fn get_vm_statistics() -> io::Result<VmStats> {
        unsafe {
            let host = mach_host_self();
            let mut vm_stats: vm_statistics64_data_t = std::mem::zeroed();
            let mut count = VM_STATISTICS64_COUNT as mach_msg_type_number_t;
            
            let result = mach2::vm_statistics::host_statistics64(
                host,
                mach2::vm_statistics::HOST_VM_INFO64,
                &mut vm_stats as *mut _ as *mut c_int,
                &mut count,
            );
            
            if result != 0 {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("host_statistics64 failed: {}", result),
                ));
            }
            
            // Get page size
            let mut page_size: libc::vm_size_t = 0;
            let result = mach2::vm::host_page_size(host, &mut page_size);
            if result != 0 {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("host_page_size failed: {}", result),
                ));
            }
            
            let page_size = page_size as u64;
            
            Ok(VmStats {
                total_memory: (vm_stats.wire_count as u64 + 
                            vm_stats.active_count as u64 + 
                            vm_stats.inactive_count as u64 + 
                            vm_stats.free_count as u64) * page_size,
                free_memory: vm_stats.free_count as u64 * page_size,
                used_memory: (vm_stats.wire_count as u64 + 
                             vm_stats.active_count as u64 + 
                             vm_stats.inactive_count as u64) * page_size,
            })
        }
    }
    
    /// Get thermal state via sysctl
    #[cfg(target_os = "macos")]
    fn get_thermal_state() -> Option<u32> {
        let mut thermal: c_int = 0;
        let mut size = std::mem::size_of::<c_int>();
        
        let name = std::ffi::CString::new("hw.thermallevels").ok()?;
        let result = unsafe {
            libc::sysctlbyname(
                name.as_ptr(),
                &mut thermal as *mut _ as *mut c_void,
                &mut size,
                std::ptr::null_mut(),
                0,
            )
        };
        
        if result == 0 {
            Some(thermal as u32)
        } else {
            None
        }
    }
    
    /// Get memory pressure via sysctl
    #[cfg(target_os = "macos")]
    fn get_memory_pressure() -> Option<u32> {
        // Memory pressure can be derived from swap usage or vm pressure level
        // This is a simplified placeholder
        // Real implementation would query vm.memory_pressure
        None
    }
}

/// VM statistics structure
#[derive(Debug, Clone, Default)]
struct VmStats {
    total_memory: u64,
    free_memory: u64,
    used_memory: u64,
}

/// Check if running on Apple Silicon
#[cfg(target_os = "macos")]
pub fn is_apple_silicon() -> bool {
    use std::ffi::CString;
    
    let mut cpu_brand: [c_char; 128] = [0; 128];
    let mut size = cpu_brand.len();
    
    let name = match CString::new("machdep.cpu.brand_string") {
        Ok(n) => n,
        Err(_) => return false,
    };
    
    let result = unsafe {
        libc::sysctlbyname(
            name.as_ptr(),
            cpu_brand.as_mut_ptr() as *mut c_void,
            &mut size,
            std::ptr::null_mut(),
            0,
        )
    };
    
    if result != 0 {
        return false;
    }
    
    // Convert to string and check for Apple
    let brand = unsafe {
        std::ffi::CStr::from_ptr(cpu_brand.as_ptr())
            .to_string_lossy()
    };
    
    brand.contains("Apple")
}

/// Non-macOS fallback
#[cfg(not(target_os = "macos"))]
pub fn is_apple_silicon() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_system_metrics_default() {
        let metrics = SystemMetrics::default();
        assert_eq!(metrics.cpu_percent, 0.0);
        assert_eq!(metrics.memory_pressure, 0);
        assert_eq!(metrics.thermal_state, 0);
    }
    
    #[test]
    fn test_is_apple_silicon() {
        // Should not panic on any platform
        let _ = is_apple_silicon();
    }
}
