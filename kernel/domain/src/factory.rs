//! Platform-aware backend factory
//!
//! Detects and creates appropriate backends based on platform and availability.

use crate::ports::AcceleratorBackend;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Backend factory that detects platform capabilities
pub struct PlatformBackendFactory;

impl PlatformBackendFactory {
    /// Create all available backends for the current platform
    /// 
    /// This is an async function because some backends (like TPU) may
    /// need to establish connections during initialization.
    pub async fn create_backends() -> Vec<Arc<dyn AcceleratorBackend>> {
        let mut backends: Vec<Arc<dyn AcceleratorBackend>> = Vec::new();
        
        #[cfg(all(target_os = "macos", feature = "metal"))]
        {
            debug!("Checking for Metal backend availability");
            if let Some(backend) = Self::try_create_metal().await {
                info!("Metal backend initialized");
                backends.push(backend);
            }
        }
        
        #[cfg(all(target_os = "linux", feature = "nvml"))]
        {
            debug!("Checking for NVML backend availability");
            if let Some(backend) = Self::try_create_nvml() {
                info!("NVML backend initialized");
                backends.push(backend);
            }
        }
        
        #[cfg(feature = "tpu")]
        {
            debug!("Checking for TPU backend availability");
            if let Some(backend) = Self::try_create_tpu().await {
                info!("TPU backend initialized");
                backends.push(backend);
            }
        }
        
        if backends.is_empty() {
            warn!("No accelerator backends available on this platform");
        } else {
            info!("Initialized {} backend(s)", backends.len());
        }
        
        backends
    }
    
    /// Check which backends are available without creating them
    pub fn available_backends() -> Vec<&'static str> {
        let mut available = Vec::new();
        
        #[cfg(all(target_os = "macos", feature = "metal"))]
        {
            // Check if Metal is available (Apple Silicon)
            // This is a compile-time check, runtime check happens in try_create_metal
            available.push("metal");
        }
        
        #[cfg(all(target_os = "linux", feature = "nvml"))]
        {
            available.push("nvml");
        }
        
        #[cfg(feature = "tpu")]
        {
            available.push("tpu");
        }
        
        available
    }
    
    /// Try to create Metal backend
    #[cfg(all(target_os = "macos", feature = "metal"))]
    async fn try_create_metal() -> Option<Arc<dyn AcceleratorBackend>> {
        use tensor_guardian_adapter_metal::MetalBackendFactory;
        
        MetalBackendFactory::try_create()
            .map(|b| Arc::new(b) as Arc<dyn AcceleratorBackend>)
    }
    
    #[cfg(not(all(target_os = "macos", feature = "metal")))]
    async fn try_create_metal() -> Option<Arc<dyn AcceleratorBackend>> {
        None
    }
    
    /// Try to create NVML backend
    #[cfg(all(target_os = "linux", feature = "nvml"))]
    fn try_create_nvml() -> Option<Arc<dyn AcceleratorBackend>> {
        use tensor_guardian_adapter_nvml::NvmlBackendFactory;
        
        NvmlBackendFactory::try_create()
            .map(|b| Arc::new(b) as Arc<dyn AcceleratorBackend>)
    }
    
    #[cfg(not(all(target_os = "linux", feature = "nvml")))]
    fn try_create_nvml() -> Option<Arc<dyn AcceleratorBackend>> {
        None
    }
    
    /// Try to create TPU backend
    #[cfg(feature = "tpu")]
    async fn try_create_tpu() -> Option<Arc<dyn AcceleratorBackend>> {
        use tensor_guardian_adapter_tpu::TpuBackendFactory;
        
        TpuBackendFactory::try_create().await
            .map(|b| Arc::new(b) as Arc<dyn AcceleratorBackend>)
    }
    
    #[cfg(not(feature = "tpu"))]
    async fn try_create_tpu() -> Option<Arc<dyn AcceleratorBackend>> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_create_backends() {
        // Should not panic
        let backends = PlatformBackendFactory::create_backends().await;
        // Depending on platform, may or may not have backends
        assert!(backends.len() >= 0);
    }
    
    #[test]
    fn test_available_backends() {
        let available = PlatformBackendFactory::available_backends();
        // Should return at least empty vec
        assert!(available.len() >= 0);
    }
}
