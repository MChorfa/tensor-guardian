//! Business policies and validation rules for tensor-guardian

use tensor_guardian_domain::{
    aggregates::Accelerator,
    entities::Metric,
    value_objects::{MetricType, Timestamp},
};

/// Policy engine for validating metrics and operations
pub struct PolicyEngine;

impl PolicyEngine {
    pub fn new() -> Self {
        Self
    }

    /// Validate a metric against defined policies
    pub fn validate_metric(&self, metric: &Metric) -> Result<(), PolicyError> {
        // Check for valid ranges
        match metric.metric_type {
            MetricType::Utilization => {
                if let Some(value) = metric.value.as_f64() {
                    if value < 0.0 || value > 100.0 {
                        return Err(PolicyError::OutOfRange {
                            metric: "utilization",
                            value,
                            min: 0.0,
                            max: 100.0,
                        });
                    }
                }
            }
            MetricType::Temperature => {
                if let Some(value) = metric.value.as_f64() {
                    if value > 120.0 {
                        return Err(PolicyError::CriticalTemperature(value));
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Check if accelerator health is within acceptable bounds
    pub fn check_accelerator_health(&self, accel: &Accelerator) -> HealthStatus {
        let last_seen_duration = Timestamp::now().nanos - accel.last_seen.nanos;
        let seconds_since_seen = last_seen_duration / 1_000_000_000;

        if seconds_since_seen > 60 {
            HealthStatus::Stale
        } else if !accel.enabled {
            HealthStatus::Disabled
        } else {
            HealthStatus::Healthy
        }
    }
}

impl Default for PolicyEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PolicyError {
    #[error("Metric {metric} out of range: {value} (expected {min}-{max})")]
    OutOfRange {
        metric: &'static str,
        value: f64,
        min: f64,
        max: f64,
    },

    #[error("Critical temperature detected: {0}°C")]
    CriticalTemperature(f64),

    #[error("Policy violation: {0}")]
    Violation(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Stale,
    Disabled,
}

/// Sampling policy configuration
pub struct SamplingPolicy {
    pub interval_ms: u64,
    pub max_retries: u32,
    pub timeout_ms: u64,
}

impl Default for SamplingPolicy {
    fn default() -> Self {
        Self {
            interval_ms: 1000,
            max_retries: 3,
            timeout_ms: 5000,
        }
    }
}

/// Retention policy for metric data
pub enum RetentionPolicy {
    /// Keep last N samples
    LastN(usize),
    /// Keep samples for duration (in seconds)
    Duration(u64),
    /// Unlimited retention
    Unlimited,
}
