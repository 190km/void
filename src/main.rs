mod app;
mod canvas;
mod command_palette;
mod config;
mod shortcuts;
mod sidebar;
mod state;
mod terminal;
mod theme;
mod utils;

use anyhow::Result;

fn main() -> Result<()> {
    env_logger::init();
    log::info!("Starting Void terminal...");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Void")
            .with_inner_size([1024.0, 640.0])
            .with_min_inner_size([640.0, 400.0]),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };

    eframe::run_native(
        "Void",
        options,
        Box::new(|cc| Ok(Box::new(app::VoidApp::new(cc)))),
    )
    .map_err(|e| anyhow::anyhow!("eframe error: {}", e))?;

    Ok(())
}
