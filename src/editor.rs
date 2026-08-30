//! Editor cursor helpers.

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
