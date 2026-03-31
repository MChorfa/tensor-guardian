use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Sampling configuration
    pub sampling: SamplingConfig,
    
    /// Backend configurations
    pub backends: HashMap<String, BackendConfig>,
    
    /// Export configuration
    pub export: ExportConfig,
    
    /// TUI configuration
    pub tui: TuiConfig,
    
    /// Logging configuration
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamplingConfig {
    /// Default sampling interval in milliseconds
    pub interval_ms: u64,
    
    /// Retention policy
    pub retention: RetentionPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RetentionPolicy {
    /// Keep last N samples
    LastN(usize),
    /// Keep samples for duration
    Duration(String),
    /// Unlimited
    Unlimited,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendConfig {
    /// Whether backend is enabled
    pub enabled: bool,
    
    /// Backend-specific settings
    #[serde(flatten)]
    pub settings: HashMap<String, toml::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportConfig {
    /// Prometheus exporter configuration
    pub prometheus: Option<PrometheusConfig>,
    
    /// OpenTelemetry configuration
    pub opentelemetry: Option<OpenTelemetryConfig>,
    
    /// CSV logging configuration
    pub csv: Option<CsvConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrometheusConfig {
    /// Port to listen on
    pub port: u16,
    
    /// Host to bind to
    pub host: String,
    
    /// Path for metrics endpoint
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenTelemetryConfig {
    /// OTLP endpoint
    pub endpoint: String,
    
    /// Export interval in seconds
    pub interval_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsvConfig {
    /// Output file path
    pub path: String,
    
    /// Whether to include headers
    pub headers: bool,
    
    /// Flush interval
    pub flush_interval_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuiConfig {
    /// Color scheme
    pub color_scheme: String,
    
    /// Show history chart
    pub show_history: bool,
    
    /// History buffer size
    pub history_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level
    pub level: String,
    
    /// Log to file
    pub file: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        let mut backends = HashMap::new();
        backends.insert(
            "nvml".to_string(),
            BackendConfig {
                enabled: true,
                settings: HashMap::new(),
            },
        );
        
        Self {
            sampling: SamplingConfig {
                interval_ms: 1000,
                retention: RetentionPolicy::LastN(1000),
            },
            backends,
            export: ExportConfig {
                prometheus: Some(PrometheusConfig {
                    port: 9101,
                    host: "0.0.0.0".to_string(),
                    path: "/metrics".to_string(),
                }),
                opentelemetry: None,
                csv: None,
            },
            tui: TuiConfig {
                color_scheme: "default".to_string(),
                show_history: true,
                history_size: 20,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                file: None,
            },
        }
    }
}

impl Config {
    pub fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&contents)?;
        Ok(config)
    }
    
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        let contents = toml::to_string_pretty(self)?;
        std::fs::write(path, contents)?;
        Ok(())
    }
}
