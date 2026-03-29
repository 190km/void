#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod canvas;
mod command_palette;
mod deeplink;
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

    // Register void:// protocol handler on this system (idempotent, silent)
    deeplink::register::ensure_registered();

    // Check for void:// deep-link URL passed as CLI argument
    let url_arg = std::env::args().nth(1).filter(|a| a.starts_with("void://"));

    // If another instance is already running, send the URL to it and exit
    if let Some(ref url) = url_arg {
        if deeplink::ipc::try_send_to_running(url) {
            log::info!("Sent deep-link to running instance: {url}");
            return Ok(());
        }
    }

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
        Box::new(move |cc| Ok(Box::new(app::VoidApp::new(cc, url_arg)))),
    )
    .map_err(|e| anyhow::anyhow!("eframe error: {}", e))?;

    Ok(())
}
