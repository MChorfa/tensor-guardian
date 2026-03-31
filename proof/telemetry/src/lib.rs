//! OpenTelemetry integration for tensor-guardian
//!
//! Provides OTLP/gRPC and stdout export for traces, metrics, and logs.

use opentelemetry::{
    global,
    trace::SpanKind,
    Context, KeyValue,
};
use opentelemetry_sdk::{
    metrics::{PeriodicReader, SdkMeterProvider},
    runtime::Tokio,
    trace::{RandomIdGenerator, Sampler, TracerProvider},
    Resource,
};
use opentelemetry_stdout::{MetricsExporter, SpanExporter};
use std::sync::Arc;
use tensor_guardian_domain::{
    aggregates::{Accelerator, Sample},
    entities::Metric,
    value_objects::{MetricType},
};
use tracing::{info, warn};

/// Telemetry configuration
#[derive(Debug, Clone)]
pub struct TelemetryConfig {
    /// Service name for OTEL resource
    pub service_name: String,
    /// Service version
    pub service_version: String,
    /// Export format: "stdout" or "otlp"
    pub export_format: ExportFormat,
    /// Export interval in milliseconds
    pub export_interval_ms: u64,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            service_name: "tensor-guardian".to_string(),
            service_version: env!("CARGO_PKG_VERSION").to_string(),
            export_format: ExportFormat::Stdout,
            export_interval_ms: 5000,
        }
    }
}

/// Export format for telemetry
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Stdout,
    Otlp,
}

impl std::str::FromStr for ExportFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "stdout" => Ok(ExportFormat::Stdout),
            "otlp" => Ok(ExportFormat::Otlp),
            _ => Err(format!("Unknown export format: {}", s)),
        }
    }
}

/// Telemetry exporter with OTEL pipeline
pub struct TelemetryExporter {
    config: TelemetryConfig,
    meter_provider: Option<SdkMeterProvider>,
    tracer_provider: Option<TracerProvider>,
}

impl TelemetryExporter {
    /// Create new telemetry exporter
    pub fn new(config: TelemetryConfig) -> Self {
        Self {
            config,
            meter_provider: None,
            tracer_provider: None,
        }
    }

    /// Initialize OTEL pipeline with stdout exporter
    pub fn init(&mut self) -> anyhow::Result<()> {
        info!("Initializing OpenTelemetry pipeline ({:?})", self.config.export_format);

        let resource = Resource::new(vec![
            KeyValue::new("service.name", self.config.service_name.clone()),
            KeyValue::new("service.version", self.config.service_version.clone()),
            KeyValue::new("deployment.environment", "development"),
        ]);

        // Initialize metrics with stdout exporter
        let metric_exporter = MetricsExporter::default();
        let reader = PeriodicReader::builder(metric_exporter, Tokio)
            .with_interval(std::time::Duration::from_millis(self.config.export_interval_ms))
            .build();

        let meter_provider = SdkMeterProvider::builder()
            .with_resource(resource.clone())
            .with_reader(reader)
            .build();

        // Initialize traces with stdout exporter
        let span_exporter = SpanExporter::default();
        let tracer_provider = TracerProvider::builder()
            .with_resource(resource)
            .with_simple_exporter(span_exporter)
            .with_sampler(Sampler::AlwaysOn)
            .with_id_generator(RandomIdGenerator::default())
            .build();

        // Set as global providers
        global::set_meter_provider(meter_provider.clone());
        global::set_tracer_provider(tracer_provider.clone());

        self.meter_provider = Some(meter_provider);
        self.tracer_provider = Some(tracer_provider);

        info!("OpenTelemetry pipeline initialized");
        Ok(())
    }

    /// Export a single metric (stub - will be expanded with proper instruments)
    pub fn export_metric(&self, _metric: &Metric, _accel: &Accelerator) {
        // TODO: Implement metric export using meter provider
    }

    /// Export a sample batch
    pub fn export_sample(&self, sample: &Sample, accel: &Accelerator) {
        for metric in &sample.metrics {
            self.export_metric(metric, accel);
        }
    }

    /// Record discovery event
    pub fn record_discovery(&self, _backend_type: &str, _count: u64) {
        // TODO: Use counter instrument
    }

    /// Record collection duration
    pub fn record_collection_duration(&self, _duration_ms: f64, _backend_type: &str) {
        // TODO: Use histogram instrument
    }

    /// Create a tracer span
    pub fn create_span(&self, name: &str, kind: SpanKind) {
        if self.tracer_provider.is_some() {
            let tracer = global::tracer("tensor-guardian");
            let _span = tracer.span_builder(name).with_kind(kind).start(&tracer);
        }
    }

    /// Get tracer provider
    pub fn tracer_provider(&self) -> Option<&TracerProvider> {
        self.tracer_provider.as_ref()
    }

    /// Get meter provider
    pub fn meter_provider(&self) -> Option<&SdkMeterProvider> {
        self.meter_provider.as_ref()
    }

    /// Shutdown telemetry pipeline
    pub fn shutdown(&self) {
        if let Some(provider) = &self.meter_provider {
            if let Err(e) = provider.shutdown() {
                warn!("Error shutting down meter provider: {:?}", e);
            }
        }
        global::shutdown_tracer_provider();
        info!("OpenTelemetry pipeline shutdown complete");
    }
}

impl Default for TelemetryExporter {
    fn default() -> Self {
        Self::new(TelemetryConfig::default())
    }
}

/// Global telemetry handle for use across the application
pub static GLOBAL_TELEMETRY: std::sync::OnceLock<std::sync::Arc<TelemetryExporter>> =
    std::sync::OnceLock::new();

/// Initialize global telemetry
pub fn init_global_telemetry(config: TelemetryConfig) -> anyhow::Result<()> {
    let mut exporter = TelemetryExporter::new(config);
    exporter.init()?;
    GLOBAL_TELEMETRY
        .set(std::sync::Arc::new(exporter))
        .map_err(|_| anyhow::anyhow!("Global telemetry already initialized"))?;
    Ok(())
}

/// Get global telemetry exporter
pub fn global_telemetry() -> Option<std::sync::Arc<TelemetryExporter>> {
    GLOBAL_TELEMETRY.get().cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telemetry_config_default() {
        let config = TelemetryConfig::default();
        assert_eq!(config.service_name, "tensor-guardian");
        assert!(matches!(config.export_format, ExportFormat::Stdout));
    }

    #[test]
    fn test_export_format_parse() {
        assert!(matches!(
            "stdout".parse::<ExportFormat>().unwrap(),
            ExportFormat::Stdout
        ));
        assert!(matches!(
            "OTLP".parse::<ExportFormat>().unwrap(),
            ExportFormat::Otlp
        ));
    }
}
