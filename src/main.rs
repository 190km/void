#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod canvas;
mod command_palette;
mod panel;
mod shortcuts;
mod sidebar;
mod state;
mod terminal;
mod theme;
mod update;
mod utils;

use anyhow::Result;
use std::sync::Arc;

fn main() -> Result<()> {
    env_logger::init();
    log::info!("Starting Void terminal...");

    let icon = {
        let png = include_bytes!("../assets/icon.png");
        let img = image::load_from_memory(png)
            .expect("Failed to load app icon")
            .to_rgba8();
        let (w, h) = img.dimensions();
        egui::IconData {
            rgba: img.into_raw(),
            width: w,
            height: h,
        }
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(format!("Void | v{}", env!("CARGO_PKG_VERSION")))
            .with_inner_size([1024.0, 640.0])
            .with_min_inner_size([640.0, 400.0])
            .with_icon(Arc::new(icon)),
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
