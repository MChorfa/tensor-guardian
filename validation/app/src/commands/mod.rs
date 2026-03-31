use async_trait::async_trait;
use tensor_guardian_domain::{
    aggregates::{Accelerator, Sample},
    ports::{AcceleratorBackend, AcceleratorRepository, EventBus},
    value_objects::AcceleratorId,
};
use crate::AppResult;
use std::sync::Arc;
use tracing::{info, warn, error};

/// Command to discover all available accelerators
#[derive(Debug, Clone)]
pub struct DiscoverAccelerators {
    pub backend_filter: Option<Vec<String>>,
}

/// Handler for DiscoverAccelerators command
pub struct DiscoverAcceleratorsHandler {
    backends: Vec<Arc<dyn AcceleratorBackend>>,
    repository: Arc<dyn AcceleratorRepository>,
    event_bus: Arc<dyn EventBus>,
}

impl DiscoverAcceleratorsHandler {
    pub fn new(
        backends: Vec<Arc<dyn AcceleratorBackend>>,
        repository: Arc<dyn AcceleratorRepository>,
        event_bus: Arc<dyn EventBus>,
    ) -> Self {
        Self {
            backends,
            repository,
            event_bus,
        }
    }

    pub async fn handle(&self, cmd: DiscoverAccelerators) -> AppResult<Vec<AcceleratorId>> {
        info!("Starting accelerator discovery");
        
        let mut discovered = Vec::new();
        
        for backend in &self.backends {
            let backend_type = backend.backend_type();
            
            // Check if backend is filtered
            if let Some(ref filter) = cmd.backend_filter {
                if !filter.contains(&backend_type.to_string()) {
                    continue;
                }
            }
            
            // Check backend health
            let health = backend.health().await;
            if !health.is_healthy() {
                warn!("Backend {} is not healthy: {:?}", backend_type, health);
                continue;
            }
            
            // Attempt discovery
            match backend.discover().await {
                Ok(accelerators) => {
                    info!("Backend {} discovered {} accelerators", backend_type, accelerators.len());
                    
                    for accel in accelerators {
                        let id = accel.id;
                        
                        // Save to repository
                        if let Err(e) = self.repository.save(&accel).await {
                            error!("Failed to save accelerator {}: {}", id, e);
                            continue;
                        }
                        
                        // Publish discovery events
                        for event in accel.events.clone() {
                            if let Err(e) = self.event_bus.publish(event).await {
                                warn!("Failed to publish event: {}", e);
                            }
                        }
                        
                        discovered.push(id);
                    }
                }
                Err(e) => {
                    warn!("Backend {} discovery failed: {}", backend_type, e);
                }
            }
        }
        
        info!("Discovery complete: {} accelerators found", discovered.len());
        Ok(discovered)
    }
}

/// Command to collect metrics from an accelerator
#[derive(Debug, Clone)]
pub struct CollectMetrics {
    pub accelerator_id: AcceleratorId,
    pub sensor_filter: Option<Vec<String>>,
}

/// Handler for CollectMetrics command
pub struct CollectMetricsHandler {
    backends: Vec<Arc<dyn AcceleratorBackend>>,
    repository: Arc<dyn AcceleratorRepository>,
    event_bus: Arc<dyn EventBus>,
}

impl CollectMetricsHandler {
    pub fn new(
        backends: Vec<Arc<dyn AcceleratorBackend>>,
        repository: Arc<dyn AcceleratorRepository>,
        event_bus: Arc<dyn EventBus>,
    ) -> Self {
        Self {
            backends,
            repository,
            event_bus,
        }
    }

    pub async fn handle(&self, cmd: CollectMetrics) -> AppResult<Sample> {
        // Find accelerator
        let mut accelerator = self.repository
            .find_by_id(cmd.accelerator_id)
            .await?
            .ok_or_else(|| tensor_guardian_domain::DomainError::AcceleratorNotFound(
                cmd.accelerator_id.to_string()
            ))?;
        
        // Find appropriate backend
        let backend = self.backends
            .iter()
            .find(|b| b.backend_type() == accelerator.backend_type)
            .ok_or_else(|| crate::AppError::BackendNotAvailable(
                accelerator.backend_type.clone()
            ))?;
        
        // Collect metrics
        let sample = backend.collect(&accelerator).await?;
        
        // Update accelerator last_seen
        accelerator.update_last_seen();
        self.repository.save(&accelerator).await?;
        
        // Publish collection events
        for metric in &sample.metrics {
            let event = tensor_guardian_domain::events::DomainEvent::MetricCollected(
                tensor_guardian_domain::events::MetricCollected {
                    metric_id: metric.id,
                    sensor_id: metric.sensor_id,
                    accelerator_id: cmd.accelerator_id,
                    metric_type: metric.metric_type.clone(),
                    timestamp: metric.timestamp,
                }
            );
            
            if let Err(e) = self.event_bus.publish(event).await {
                warn!("Failed to publish metric event: {}", e);
            }
        }
        
        Ok(sample)
    }
}

/// Command to attach an eBPF probe
#[derive(Debug, Clone)]
pub struct AttachProbe {
    pub probe_name: String,
}

/// Handler for AttachProbe command
pub struct AttachProbeHandler;

impl AttachProbeHandler {
    pub fn new() -> Self {
        Self
    }

    pub async fn handle(&self, _cmd: AttachProbe) -> AppResult<()> {
        // TODO: Implement probe attachment
        Ok(())
    }
}

impl Default for AttachProbeHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Command to configure sampling settings
#[derive(Debug, Clone)]
pub struct ConfigureSampling {
    pub interval_ms: u64,
}

/// Handler for ConfigureSampling command
pub struct ConfigureSamplingHandler;

impl ConfigureSamplingHandler {
    pub fn new() -> Self {
        Self
    }

    pub async fn handle(&self, _cmd: ConfigureSampling) -> AppResult<()> {
        // TODO: Implement sampling configuration
        Ok(())
    }
}

impl Default for ConfigureSamplingHandler {
    fn default() -> Self {
        Self::new()
    }
}
