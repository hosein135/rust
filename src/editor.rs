//! Code editor panel with line numbers and syntax paint.

use crate::syntax::{self, Span};
use crate::theme;
use egui::{self, FontId, Sense, TextEdit, Ui, Vec2};

pub struct EditorOut {
    pub changed: bool,
    pub cursor: usize,
}

pub fn show_editor(ui: &mut Ui, content: &mut String, cursor: &mut usize, font_size: f32) -> EditorOut {
    let mut changed = false;
    let line_count = content.lines().count().max(1);
    let gutter_w = 12.0 + (line_count.to_string().len() as f32) * font_size * 0.6;

    let available = ui.available_size();
    egui::ScrollArea::both()
        .id_salt("editor_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_min_size(available);
            ui.horizontal_top(|ui| {
                // Gutter
                let (gutter_rect, _) = ui.allocate_exact_size(
                    Vec2::new(gutter_w, (line_count as f32) * (font_size + 4.0) + 8.0),
                    Sense::hover(),
                );
                let painter = ui.painter_at(gutter_rect);
                painter.rect_filled(gutter_rect, 0.0, theme::PANEL);
                let mut y = gutter_rect.top() + 4.0;
                for (i, _) in content.split_inclusive('\n').enumerate() {
                    painter.text(
                        egui::pos2(gutter_rect.right() - 6.0, y),
                        egui::Align2::RIGHT_TOP,
                        format!("{}", i + 1),
                        FontId::monospace(font_size * 0.9),
                        theme::TEXT_DIM,
                    );
                    y += font_size + 4.0;
                }

                ui.separator();

                let edit_id = ui.make_persistent_id("verilog_editor");
                let fs = font_size;

                let text_edit = TextEdit::multiline(content)
                    .id(edit_id)
                    .font(FontId::monospace(font_size))
                    .desired_width(f32::INFINITY)
                    .frame(false)
                    .code_editor()
                    .layouter(&mut move |ui: &Ui, text: &str, wrap_width: f32| {
                        let spans = syntax::highlight(text);
                        layout_highlighted(ui, text, wrap_width, fs, &spans)
                    });

                let response = ui.add_sized(
                    Vec2::new(
                        (available.x - gutter_w - 12.0).max(200.0),
                        (line_count as f32) * (font_size + 4.0) + 8.0,
                    ),
                    text_edit,
                );

                if response.changed() {
                    changed = true;
                }

                if let Some(te) = TextEdit::load_state(ui.ctx(), edit_id) {
                    if let Some(range) = te.cursor.char_range() {
                        *cursor = range.primary.index;
                    }
                }
            });
        });

    EditorOut {
        changed,
        cursor: *cursor,
    }
}

fn layout_highlighted(
    ui: &Ui,
    text: &str,
    wrap_width: f32,
    font_size: f32,
    spans: &[Span],
) -> std::sync::Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = wrap_width;

    if spans.is_empty() {
        job.append(
            text,
            0.0,
            egui::TextFormat {
                font_id: FontId::monospace(font_size),
                color: theme::IDENT,
                ..Default::default()
            },
        );
    } else {
        for span in spans {
            let slice = &text[span.start..span.end];
            job.append(
                slice,
                0.0,
                egui::TextFormat {
                    font_id: FontId::monospace(font_size),
                    color: span.kind.color(),
                    ..Default::default()
                },
            );
        }
    }

    ui.fonts(|f| f.layout_job(job))
}

pub fn cursor_line_col(content: &str, cursor: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut col = 1usize;
    for (i, ch) in content.chars().enumerate() {
        if i >= cursor {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}
