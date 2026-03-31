pub mod app;
pub mod components;
pub mod events;
pub mod ui;

pub use app::TuiApp;

use anyhow::Result;
use tensor_guardian_domain::aggregates::Accelerator;

/// Initialize and run the TUI
pub async fn run(
    accelerators: Vec<Accelerator>,
    refresh_interval_ms: u64,
) -> Result<()> {
    let mut app = TuiApp::new(accelerators, refresh_interval_ms);
    app.run().await
}
