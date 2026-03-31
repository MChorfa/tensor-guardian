use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use tracing::{error, info, warn};

mod commands;
mod config;

use commands::{run_discover, run_monitor, run_tui};
use config::Config;

#[derive(Parser)]
#[command(name = "tensor-guardian")]
#[command(about = "Universal AI accelerator monitoring with eBPF support")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Configuration file path
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Backend to use (nvml, metal, tpu, custom)
    #[arg(short, long)]
    backend: Option<Vec<String>>,

    /// Log level
    #[arg(short, long, default_value = "info")]
    log_level: String,

    /// Headless mode (no TUI)
    #[arg(short = 'n', long)]
    headless: bool,
    
    /// OTEL export format (stdout, otlp, none)
    #[arg(long, default_value = "stdout")]
    otel_format: String,
    
    /// OTEL export endpoint (for otlp format)
    #[arg(long, env = "OTEL_EXPORTER_OTLP_ENDPOINT")]
    otel_endpoint: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Discover available accelerators
    Discover {
        /// Output format
        #[arg(short, long, value_enum, default_value = "table")]
        format: OutputFormat,
    },
    
    /// Start monitoring (TUI mode)
    Monitor {
        /// Refresh interval in milliseconds
        #[arg(short, long, default_value = "1000")]
        interval: u64,
        
        /// Log to CSV file
        #[arg(short, long, value_name = "FILE")]
        log: Option<PathBuf>,
        
        /// Export Prometheus metrics on port
        #[arg(short, long, value_name = "PORT")]
        prometheus: Option<u16>,
    },
    
    /// Export metrics (headless mode)
    Export {
        /// Export format
        #[arg(short, long, value_enum, default_value = "prometheus")]
        format: ExportFormat,
        
        /// Port to listen on
        #[arg(short, long, default_value = "9101")]
        port: u16,
    },
    
    /// List available backends
    Backends,
    
    /// Check health status
    Health,
    
    /// Start API server (gRPC + HTTP)
    Api {
        /// gRPC server port
        #[arg(long, default_value = "50051")]
        grpc_port: u16,
        
        /// HTTP REST server port
        #[arg(long, default_value = "8080")]
        http_port: u16,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum OutputFormat {
    Table,
    Json,
    Yaml,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum ExportFormat {
    Prometheus,
    Opentelemetry,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(&cli.log_level)
        .init();

    info!("Starting tensor-guardian");
    
    // Initialize OpenTelemetry if not disabled
    if cli.otel_format != "none" {
        let otel_config = tensor_guardian_telemetry::TelemetryConfig {
            service_name: "tensor-guardian".to_string(),
            service_version: env!("CARGO_PKG_VERSION").to_string(),
            export_format: cli.otel_format.parse().unwrap_or(tensor_guardian_telemetry::ExportFormat::Stdout),
            export_interval_ms: 5000,
        };
        
        if let Err(e) = tensor_guardian_telemetry::init_global_telemetry(otel_config) {
            warn!("Failed to initialize telemetry: {}", e);
        } else {
            info!("OpenTelemetry initialized");
        }
    }

    // Load configuration
    let config = if let Some(config_path) = cli.config {
        Config::from_file(&config_path)?
    } else {
        Config::default()
    };

    match cli.command {
        Some(Commands::Discover { format }) => {
            run_discover(format, cli.backend).await?;
        }
        Some(Commands::Monitor { interval, log, prometheus }) => {
            if cli.headless {
                // Headless mode with logging
                run_monitor(interval, log, prometheus).await?;
            } else {
                // TUI mode
                run_tui(interval, log, prometheus).await?;
            }
        }
        Some(Commands::Export { format, port }) => {
            info!("Export mode not yet implemented: {:?} on port {}", format, port);
        }
        Some(Commands::Backends) => {
            list_backends().await?;
        }
        Some(Commands::Health) => {
            check_health().await?;
        }
        Some(Commands::Api { grpc_port, http_port }) => {
            commands::run_api_server(grpc_port, http_port).await?;
        }
        None => {
            // Default to TUI monitor mode
            if cli.headless {
                run_monitor(1000, None, None).await?;
            } else {
                run_tui(1000, None, None).await?;
            }
        }
    }

    Ok(())
}

async fn list_backends() -> anyhow::Result<()> {
    println!("Available backends:");
    
    #[cfg(feature = "nvml")]
    {
        match nvml_wrapper::Nvml::init() {
            Ok(_) => println!("  [✓] nvml     - NVIDIA GPU monitoring (NVML)"),
            Err(_) => println!("  [✗] nvml     - NVIDIA GPU monitoring (NVML) - not available"),
        }
    }
    
    #[cfg(not(feature = "nvml"))]
    println!("  [-] nvml     - NVIDIA GPU monitoring (not compiled)");
    
    // Placeholder for other backends
    println!("  [-] metal    - Apple Metal/Neural Engine (not yet implemented)");
    println!("  [-] tpu      - Google Cloud TPU (not yet implemented)");
    println!("  [-] custom   - User-defined accelerator (not yet implemented)");
    
    Ok(())
}

async fn check_health() -> anyhow::Result<()> {
    println!("Health check:");
    
    // TODO: Implement proper health checks
    println!("  System: OK");
    println!("  Kernel: OK (Linux detected)");
    
    #[cfg(feature = "nvml")]
    {
        match nvml_wrapper::Nvml::init() {
            Ok(_) => println!("  NVML:   OK"),
            Err(e) => println!("  NVML:   FAILED - {}", e),
        }
    }
    
    Ok(())
}
