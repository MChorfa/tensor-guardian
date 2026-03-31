//! Apple Neural Engine (ANE) metrics
//!
//! Provides ANE power and utilization metrics via IOKit.

#[cfg(target_os = "macos")]
use std::io;

/// Neural Engine metrics
#[derive(Debug, Clone, Default)]
pub struct AneMetrics {
    /// ANE power consumption in watts
    pub power_watts: f64,
    /// ANE utilization percentage
    pub utilization_percent: f64,
    /// ANE temperature in celsius
    pub temperature_celsius: f64,
    /// Whether ANE is available
    pub available: bool,
}

/// ANE backend
#[derive(Debug)]
pub struct AneBackend;

impl AneBackend {
    /// Create new ANE backend
    pub fn new() -> Self {
        Self
    }

    /// Check if ANE is available on this system
    #[cfg(target_os = "macos")]
    pub fn is_available() -> bool {
        // ANE is available on Apple Silicon (A14/M1 and later)
        // Check for ANE service via IOKit
        
        // For now, use Apple Silicon detection as proxy
        super::sysctl::is_apple_silicon()
    }

    /// Non-macOS fallback
    #[cfg(not(target_os = "macos"))]
    pub fn is_available() -> bool {
        false
    }

    /// Collect ANE metrics
    pub fn collect_metrics(&self) -> io::Result<AneMetrics> {
        let mut metrics = AneMetrics::default();
        
        if !Self::is_available() {
            return Ok(metrics);
        }

        metrics.available = true;

        // Try to get power metrics via IOKit
        #[cfg(target_os = "macos")]
        {
            metrics.power_watts = self.get_ane_power().unwrap_or(0.0);
            metrics.temperature_celsius = self.get_ane_temperature().unwrap_or(0.0);
        }

        Ok(metrics)
    }

    /// Get ANE power consumption
    #[cfg(target_os = "macos")]
    fn get_ane_power(&self) -> Option<f64> {
        // ANE power is exposed via AppleARMANE power management
        // This requires IOKit access to com.apple.driver.AppleARMANE
        
        // For now, estimate based on system thermal state
        // Real implementation would query IORegistry
        
        let thermal_state = super::sysctl::SystemMetrics::collect()
            .ok()?
            .thermal_state;
        
        // Estimate power based on thermal state (very rough approximation)
        // ANE typically uses 1-5W depending on workload
        let base_power = 1.0;
        let thermal_multiplier = match thermal_state {
            0 => 1.0,  // Normal
            1 => 0.8,  // Fair - throttling may occur
            2 => 0.5,  // Serious - heavy throttling
            3 => 0.2,  // Critical - severe throttling
            _ => 1.0,
        };
        
        Some(base_power * thermal_multiplier)
    }

    /// Get ANE temperature
    #[cfg(target_os = "macos")]
    fn get_ane_temperature(&self) -> Option<f64> {
        // ANE shares thermal zone with GPU on Apple Silicon
        // Try to get GPU temperature as proxy
        
        // This would require IOKit access to AppleM1IOP::AppleM1TempSensor
        // For now, return system thermal state based estimate
        
        let thermal_state = super::sysctl::SystemMetrics::collect()
            .ok()?
            .thermal_state;
        
        // Rough temperature estimates based on thermal state
        let temp = match thermal_state {
            0 => 45.0, // Normal
            1 => 60.0, // Fair
            2 => 75.0, // Serious
            3 => 85.0, // Critical
            _ => 50.0,
        };
        
        Some(temp)
    }
}

impl Default for AneBackend {
    fn default() -> Self {
        Self::new()
    }
}

/// ANE device information
#[derive(Debug, Clone)]
pub struct AneInfo {
    pub name: String,
    pub generation: AneGeneration,
    pub core_count: u32,
    pub max_tops: f64, // Trillion operations per second
}

/// ANE generation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AneGeneration {
    Gen1, // A14/M1 (11 TOPS)
    Gen2, // M1 Pro/Max/Ultra (11-22 TOPS)
    Gen3, // M2 series (15.8 TOPS)
    Gen4, // M3 series (18 TOPS)
    Gen5, // M4 series (38 TOPS)
    Unknown,
}

impl AneGeneration {
    /// Get ANE generation based on machine type
    #[cfg(target_os = "macos")]
    pub fn from_machine(machine: &str) -> Self {
        match machine {
            // M1 series - Gen1
            "MacBookAir10,1" | "MacBookAir10,2" |
            "MacBookPro17,1" | "Macmini9,1" |
            "iMac21,1" | "iMac21,2" => Self::Gen1,
            
            // M1 Pro/Max/Ultra - Gen2
            "MacBookPro18,1" | "MacBookPro18,2" |
            "MacBookPro18,3" | "MacBookPro18,4" |
            "Mac13,1" | "Mac13,2" => Self::Gen2,
            
            // M2 series - Gen3
            "Mac14,2" | "Mac14,7" |
            "Mac14,5" | "Mac14,6" |
            "Mac14,8" | "Mac14,9" |
            "Mac14,15" | "Mac14,10" => Self::Gen3,
            
            // M3 series - Gen4
            "Mac15,3" | "Mac15,4" |
            "Mac15,5" | "Mac15,6" |
            "Mac15,7" | "Mac15,8" |
            "Mac15,9" | "Mac15,10" | "Mac15,11" |
            "Mac15,12" | "Mac15,13" => Self::Gen4,
            
            // M4 series - Gen5 (as of late 2024)
            _ if machine.starts_with("Mac16,") => Self::Gen5,
            
            _ => Self::Unknown,
        }
    }

    /// Get maximum TOPS for this generation
    pub fn max_tops(&self) -> f64 {
        match self {
            Self::Gen1 => 11.0,
            Self::Gen2 => 22.0,
            Self::Gen3 => 15.8,
            Self::Gen4 => 18.0,
            Self::Gen5 => 38.0,
            Self::Unknown => 11.0,
        }
    }

    /// Get approximate core count
    pub fn core_count(&self) -> u32 {
        match self {
            Self::Gen1 => 16,
            Self::Gen2 => 16,
            Self::Gen3 => 16,
            Self::Gen4 => 16,
            Self::Gen5 => 32,
            Self::Unknown => 16,
        }
    }
}

impl Default for AneGeneration {
    fn default() -> Self {
        Self::Unknown
    }
}

/// Detect ANE generation for current machine
#[cfg(target_os = "macos")]
pub fn detect_ane_generation() -> AneGeneration {
    use libc::c_char;
    
    let mut machine: [c_char; 64] = [0; 64];
    let mut size = machine.len();
    
    let name = match std::ffi::CString::new("hw.machine") {
        Ok(n) => n,
        Err(_) => return AneGeneration::Unknown,
    };
    
    let result = unsafe {
        libc::sysctlbyname(
            name.as_ptr(),
            machine.as_mut_ptr() as *mut libc::c_void,
            &mut size,
            std::ptr::null_mut(),
            0,
        )
    };
    
    if result != 0 {
        return AneGeneration::Unknown;
    }
    
    let machine_str = unsafe {
        std::ffi::CStr::from_ptr(machine.as_ptr())
            .to_string_lossy()
    };
    
    AneGeneration::from_machine(&machine_str)
}

#[cfg(not(target_os = "macos"))]
pub fn detect_ane_generation() -> AneGeneration {
    AneGeneration::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ane_metrics_default() {
        let metrics = AneMetrics::default();
        assert_eq!(metrics.power_watts, 0.0);
        assert!(!metrics.available);
    }

    #[test]
    fn test_ane_generation_tops() {
        assert_eq!(AneGeneration::Gen1.max_tops(), 11.0);
        assert_eq!(AneGeneration::Gen2.max_tops(), 22.0);
        assert_eq!(AneGeneration::Gen3.max_tops(), 15.8);
        assert_eq!(AneGeneration::Gen4.max_tops(), 18.0);
        assert_eq!(AneGeneration::Gen5.max_tops(), 38.0);
    }

    #[test]
    fn test_ane_generation_from_machine() {
        #[cfg(target_os = "macos")]
        {
            assert_eq!(
                AneGeneration::from_machine("MacBookAir10,1"),
                AneGeneration::Gen1
            );
            assert_eq!(
                AneGeneration::from_machine("Mac14,2"),
                AneGeneration::Gen3
            );
            assert_eq!(
                AneGeneration::from_machine("Unknown"),
                AneGeneration::Unknown
            );
        }
    }

    #[test]
    fn test_ane_availability() {
        // Should not panic
        let _ = AneBackend::is_available();
    }
}
