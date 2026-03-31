use crate::OutputFormat;
use anyhow::Result;
use tensor_guardian_domain::PlatformBackendFactory;
use std::sync::Arc;
use tracing::{info, warn};

pub async fn run_discover(
    format: OutputFormat,
    _backend_filter: Option<Vec<String>>,
) -> Result<()> {
    println!("Discovering accelerators...");
    
    // Create backends using platform factory
    let backends = PlatformBackendFactory::create_backends().await;
    
    if backends.is_empty() {
        println!("No accelerators found on this system.");
        return Ok(());
    }
    
    let mut all_accelerators = Vec::new();
    
    for backend in &backends {
        match backend.discover().await {
            Ok(accelerators) => {
                all_accelerators.extend(accelerators);
            }
            Err(e) => {
                warn!("Backend {} discovery failed: {}", backend.backend_type(), e);
            }
        }
    }
    
    match format {
        OutputFormat::Table => {
            println!("\n{:<20} {:<15} {:<20} {:<30}", "Name", "Type", "Vendor", "Model");
            println!("{}", "-".repeat(85));
            for accel in &all_accelerators {
                println!(
                    "{:<20} {:<15?} {:<20} {:<30}",
                    accel.name,
                    accel.accelerator_type,
                    accel.vendor,
                    accel.model
                );
            }
        }
        OutputFormat::Json => {
            println!("{{");
            println!("  \"accelerators\": [");
            for (i, accel) in all_accelerators.iter().enumerate() {
                let comma = if i < all_accelerators.len() - 1 { "," } else { "" };
                println!(
                    "    {{\"id\": \"{}\", \"name\": \"{}\", \"type\": {:?}, \"vendor\": \"{}\"}}{}",
                    accel.id,
                    accel.name,
                    accel.accelerator_type,
                    accel.vendor,
                    comma
                );
            }
            println!("  ]");
            println!("}}");
        }
        OutputFormat::Yaml => {
            println!("accelerators:");
            for accel in &all_accelerators {
                println!("  - id: {}", accel.id);
                println!("    name: {}", accel.name);
                println!("    type: {:?}", accel.accelerator_type);
                println!("    vendor: {}", accel.vendor);
                println!("    model: {}", accel.model);
            }
        }
    }
    
    info!("Discovered {} accelerator(s)", all_accelerators.len());
    Ok(())
}

pub async fn run_monitor(
    interval: u64,
    log: Option<std::path::PathBuf>,
    _prometheus: Option<u16>,
) -> Result<()> {
    println!("Starting headless monitor (interval: {}ms)", interval);
    
    if let Some(path) = log {
        println!("Logging to: {}", path.display());
    }
    
    // Create backends
    let backends = PlatformBackendFactory::create_backends().await;
    
    if backends.is_empty() {
        println!("No accelerators available for monitoring");
        return Ok(());
    }
    
    // Discover accelerators
    let mut accelerators = Vec::new();
    for backend in &backends {
        match backend.discover().await {
            Ok(accs) => accelerators.extend(accs),
            Err(e) => warn!("Discovery failed: {}", e),
        }
    }
    
    println!("Monitoring {} accelerator(s)", accelerators.len());
    
    // TODO: Implement actual collection loop with OTEL export
    // For now, just do a single collection demo
    for accel in &accelerators {
        for backend in &backends {
            if backend.backend_type() == accel.backend_type {
                match backend.collect(accel).await {
                    Ok(sample) => {
                        println!(
                            "  {}: {} metrics collected",
                            accel.name,
                            sample.metrics.len()
                        );
                    }
                    Err(e) => {
                        warn!("Collection failed for {}: {}", accel.name, e);
                    }
                }
            }
        }
    }
    
    println!("Monitor stopped");
    Ok(())
}

pub async fn run_tui(
    interval: u64,
    _log: Option<std::path::PathBuf>,
    _prometheus: Option<u16>,
) -> Result<()> {
    // Use real discovery instead of mock data
    let backends = PlatformBackendFactory::create_backends().await;
    
    let mut accelerators = Vec::new();
    for backend in &backends {
        match backend.discover().await {
            Ok(accs) => accelerators.extend(accs),
            Err(e) => {
                warn!("Discovery failed: {}", e);
            }
        }
    }
    
    // Fallback to mock data if no real accelerators found
    if accelerators.is_empty() {
        info!("No real accelerators found, using mock data for demo");
        accelerators.push(tensor_guardian_domain::aggregates::Accelerator::new(
            "GPU 0",
            tensor_guardian_domain::value_objects::AcceleratorType::NvidiaGpu,
            "nvml",
            "NVIDIA",
            "RTX 4090",
        ));
    }
    
    // Launch the TUI
    tensor_guardian_tui::run(accelerators, interval).await
}
