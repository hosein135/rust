//! Verilog IDE — desktop application for Verilog HDL and testbenches.

mod app;
mod editor;
mod project;
mod sim;
mod templates;
mod verilog_highlighter;
mod waveform;

fn main() -> iced::Result {
    app::run()
}
