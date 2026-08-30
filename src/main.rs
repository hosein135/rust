//! Verilog IDE — desktop application for Verilog HDL and testbenches.

mod app;
mod editor;
mod project;
mod templates;

use app::VerilogIde;
use gpui::*;
use gpui_component::{Root, TitleBar, *};
use gpui_component_assets::Assets;

fn main() {
    gpui_platform::application()
        .with_assets(Assets)
        .run(move |cx| {
            gpui_component::init(cx);

            let window_options = WindowOptions {
                titlebar: Some(TitleBar::title_bar_options()),
                window_bounds: Some(WindowBounds::centered(
                    size(px(1280.), px(800.)),
                    cx,
                )),
                window_min_size: Some(gpui::Size {
                    width: px(900.),
                    height: px(560.),
                }),
                ..Default::default()
            };

            cx.spawn(async move |cx| {
                cx.open_window(window_options, |window, cx| {
                    let view = cx.new(|cx| VerilogIde::new(window, cx));
                    cx.new(|cx| Root::new(view, window, cx))
                })
                .expect("Failed to open window");
            })
            .detach();
        });
}
