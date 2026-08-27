//! Verilog IDE — desktop application for Verilog HDL and testbenches.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod editor;
mod project;
mod syntax;
mod templates;
mod theme;

use app::VerilogIde;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([900.0, 560.0])
            .with_title("Verilog IDE"),
        ..Default::default()
    };

    eframe::run_native(
        "Verilog IDE",
        options,
        Box::new(|cc| Ok(Box::new(VerilogIde::new(cc)))),
    )
}
