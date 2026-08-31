//! Editor helpers (cursor, line gutter).

use iced::widget::text;

pub const EDITOR_FONT_SIZE: f32 = 14.0;
pub const EDITOR_LINE_HEIGHT: f32 = 20.0;
pub const EDITOR_PADDING: f32 = 5.0;

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

pub fn cursor_from_line_col(content: &str, line: usize, col: usize) -> usize {
    let target_line = line + 1;
    let target_col = col + 1;
    let mut current_line = 1usize;
    let mut current_col = 1usize;
    for (i, ch) in content.chars().enumerate() {
        if current_line == target_line && current_col == target_col {
            return i;
        }
        if ch == '\n' {
            current_line += 1;
            current_col = 1;
        } else {
            current_col += 1;
        }
    }
    content.len()
}

/// Width of the line-number gutter for the given line count.
pub fn gutter_width(line_count: usize) -> f32 {
    let digits = line_count.max(1).ilog10() + 1;
    (digits.max(2) as f32 * 7.5 + 20.0).max(40.0)
}

/// Build right-aligned line numbers (`1`, `2`, …) separated by newlines.
pub fn format_line_numbers(line_count: usize) -> String {
    let count = line_count.max(1);
    let width = (count.ilog10() + 1).max(2) as usize;
    (1..=count)
        .map(|n| format!("{:>width$}", n, width = width))
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn line_number_text(line_count: usize) -> text::Text<'static> {
    text(format_line_numbers(line_count))
        .size(EDITOR_FONT_SIZE)
        .line_height(EDITOR_LINE_HEIGHT)
}
