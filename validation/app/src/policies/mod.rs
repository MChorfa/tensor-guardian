//! Policy definitions and engine

/// Policy engine for validation
pub struct PolicyEngine;

impl PolicyEngine {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PolicyEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Sampling policy
pub struct SamplingPolicy {
    pub interval_ms: u64,
}

impl Default for SamplingPolicy {
    fn default() -> Self {
        Self { interval_ms: 1000 }
    }
}

/// Retention policy
pub enum RetentionPolicy {
    TimeBased(u64),
    CountBased(usize),
    Unlimited,
}
