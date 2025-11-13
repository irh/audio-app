// hide console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;

use self::app::App;
use anyhow::{Result, anyhow};
use eframe::egui::ViewportBuilder;
use log::LevelFilter;

fn main() -> Result<()> {
    pretty_env_logger::formatted_builder()
        .filter_level(LevelFilter::Debug)
        .init();

    eframe::run_native(
        env!("PRODUCT_NAME"),
        eframe::NativeOptions {
            viewport: ViewportBuilder::default().with_inner_size([400.0, 265.0]),
            ..Default::default()
        },
        Box::new(|_cc| {
            let app = App::new()?;
            Ok(Box::new(app))
        }),
    )
    .map_err(|error| anyhow!("while running eframe app: {error}"))
}
