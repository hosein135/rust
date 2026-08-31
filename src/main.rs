//! Verilog IDE — desktop application for Verilog HDL and testbenches.

mod app;
mod editor;
mod project;
mod templates;
mod verilog_highlighter;

fn main() -> iced::Result {
    app::run()
}
