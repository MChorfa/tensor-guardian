//! Application services

/// Collection service for metrics
pub struct CollectionService;

impl CollectionService {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CollectionService {
    fn default() -> Self {
        Self::new()
    }
}

/// Sampling configuration
pub struct SamplingConfiguration {
    pub interval_ms: u64,
}

impl Default for SamplingConfiguration {
    fn default() -> Self {
        Self { interval_ms: 1000 }
    }
}

/// Collection strategy
pub enum CollectionStrategy {
    Polling,
    EventDriven,
}
