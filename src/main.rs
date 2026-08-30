//! Verilog IDE — desktop application for Verilog HDL and testbenches.

mod app;
mod editor;
mod project;
mod templates;

fn main() -> iced::Result {
    app::run()
}
