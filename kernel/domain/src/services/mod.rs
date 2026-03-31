//! Domain services for business operations
//!
//! TODO: Implement domain services when trait object issues are resolved

use crate::{
    aggregates::{Accelerator, Sample},
    value_objects::AcceleratorId,
    DomainResult,
};

/// Service for collecting metrics from accelerators
pub struct MetricCollectionService;

impl MetricCollectionService {
    pub fn new() -> Self {
        Self
    }

    /// Collect metrics from a specific accelerator
    pub async fn collect(&self, _accelerator_id: AcceleratorId) -> DomainResult<Sample> {
        // TODO: Implement collection logic
        unimplemented!()
    }
}

impl Default for MetricCollectionService {
    fn default() -> Self {
        Self::new()
    }
}

/// Service for managing accelerator lifecycle
pub struct AcceleratorManagementService;

impl AcceleratorManagementService {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AcceleratorManagementService {
    fn default() -> Self {
        Self::new()
    }
}
