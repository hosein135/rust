//! Editor tab helpers.

use gpui_component::input::Position;

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

pub fn position_from_cursor(content: &str, cursor: usize) -> Position {
    let (line, col) = cursor_line_col(content, cursor);
    Position::new(line.saturating_sub(1) as u32, col.saturating_sub(1) as u32)
}

pub fn cursor_from_position(content: &str, position: Position) -> usize {
    let target_line = position.line as usize + 1;
    let target_col = position.character as usize + 1;
    let mut line = 1usize;
    let mut col = 1usize;
    for (i, ch) in content.chars().enumerate() {
        if line == target_line && col == target_col {
            return i;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    content.len()
}
