use crossterm::{
    QueueableCommand,
    cursor::MoveTo,
    event::KeyEvent,
    execute,
    style::{Color, Print, SetBackgroundColor, SetForegroundColor},
    terminal::{Clear, ClearType},
};
use log::*;
use std::cmp::{max, min};
use std::io::{self, Write, stdin, stdout};
use strip_ansi_escapes::strip;

const PROMPT_CURSOR_OFFSET: usize = 1;
const SCROLLOFF: usize = 4;
const SCROLL_JUMP: usize = 10;

enum ScrollDirection {
    Up,
    Down,
}

enum SidewaysDirection {
    Left,
    Right,
}

pub struct Vec2<T> {
    x: T,
    y: T,
}
impl<T> Vec2<T> {
    pub fn new(x: T, y: T) -> Self {
        Vec2 { x, y }
    }
}

pub struct Selection {
    start: Vec2<usize>,
    end: Vec2<usize>,
}

impl Selection {
    pub fn new(start: Vec2<usize>, end: Vec2<usize>) -> Self {
        Selection { start, end }
    }

    pub fn with_coords(start_x: usize, start_y: usize, end_x: usize, end_y: usize) -> Self {
        Selection {
            start: Vec2::new(start_x, start_y),
            end: Vec2::new(end_x, end_y),
        }
    }
}

pub struct ScrollbackBuffer {
    lines: Vec<String>,
    text_lines: Vec<String>, // Lines without escape sequences
    cursor_x: usize,
    cursor_y: usize,
    wish_cursor_x: usize,
    term_width: usize,
    term_height: usize,
    viewport_start: usize,
    viewport_end: usize,
    selection: Option<Selection>, // We assume that start is always before end
}

impl ScrollbackBuffer {
    pub fn new() -> io::Result<Self> {
        let mut raw_lines = Vec::<String>::new();
        let mut text_lines = Vec::<String>::new();
        for line in stdin().lines() {
            let line = line?;
            let stripped = String::from_utf8_lossy(&strip(line.as_bytes())).into_owned();
            text_lines.push(stripped);
            raw_lines.push(line);
        }
        let (term_width, term_height) = crossterm::terminal::size()?;

        // The scrollback may contain empty lines at the end
        let mut last_non_empty_line_idx = raw_lines.len().saturating_sub(1);
        while last_non_empty_line_idx > 0 && raw_lines[last_non_empty_line_idx].is_empty() {
            last_non_empty_line_idx -= 1;
        }
        raw_lines.truncate(last_non_empty_line_idx + 1);
        text_lines.truncate(last_non_empty_line_idx + 1);

        let cursor_x = text_lines
            .last()
            .map(|l| l.chars().count().saturating_sub(1))
            .unwrap_or(0)
            + PROMPT_CURSOR_OFFSET;

        let cursor_y = min(
            term_height.saturating_sub(1) as usize,
            raw_lines.len().saturating_sub(1),
        );

        Ok(Self {
            cursor_x,
            cursor_y,

            wish_cursor_x: cursor_x,

            term_width: term_width as usize,
            term_height: term_height as usize,

            viewport_start: raw_lines.len().saturating_sub(term_height as usize),
            viewport_end: raw_lines.len().saturating_sub(1),

            lines: raw_lines,
            text_lines,

            // selection: None,
            selection: Some(Selection::with_coords(2, 0, 3, 4)), // DEBUG:
        })
    }

    // BUG: When selection start is outside of the viewport but the selection is
    // visible, it renders nonsense
    pub fn draw(&self) -> io::Result<()> {
        let mut out = stdout();
        out.queue(Clear(ClearType::All))?;

        for (i, line) in self.lines[self.viewport_start..self.viewport_end + 1]
            .iter()
            .enumerate()
        {
            out.queue(MoveTo(0, i as u16))?;
            out.queue(Print(line))?;
        }

        match &self.selection {
            Some(sel) => {
                if sel.end.y >= self.viewport_start && sel.start.y <= self.viewport_end {
                    let sel_physical_y_start = sel.start.y as isize - self.viewport_start as isize;
                    let mut sel_physical_y_end = sel.end.y.saturating_sub(self.viewport_start);

                    if sel_physical_y_start < 0 {
                        out.queue(MoveTo(0, 0))?;
                    } else {
                        debug!("MoveTo({}, {})", sel.start.x, sel_physical_y_start);
                        out.queue(MoveTo(sel.start.x as u16, sel_physical_y_start as u16))?;
                    }

                    out.queue(SetForegroundColor(Color::Black))?;
                    out.queue(SetBackgroundColor(Color::Yellow))?;

                    if sel.start.y == sel.end.y {
                        let text_line = &self.text_lines[sel.start.y];
                        let start_idx = get_utf_index(text_line, sel.start.x);
                        let end_idx = get_utf_index(text_line, sel.end.x + 1);
                        out.queue(Print(&text_line[start_idx..end_idx]))?;
                        out.queue(SetForegroundColor(Color::Reset))?;
                        out.queue(SetBackgroundColor(Color::Reset))?;
                    } else {
                        let mut y_idx = min(sel_physical_y_start, 0) as usize;
                        if sel_physical_y_start >= 0 {
                            y_idx = sel_physical_y_start.wrapping_abs() as usize + 1;
                            let text_line = &self.text_lines[sel.start.y];
                            let start_idx = get_utf_index(text_line, sel.start.x);
                            out.queue(Print(&text_line[start_idx..]))?;
                        }

                        let loop_start = sel.start.y
                            + ((sel_physical_y_start < 0) as usize
                                * (sel_physical_y_start).wrapping_abs() as usize)
                            + ((sel_physical_y_start >= 0) as usize * 1);

                        // This loop covers the full selection without the first and last lines
                        for (i, line) in self.text_lines[loop_start..sel.end.y].iter().enumerate() {
                            out.queue(MoveTo(0, (y_idx + i) as u16))?;
                            out.queue(Print(line))?;
                        }
                        // TODO: adapt for when this is outside of the viewport
                        let text_line = &self.text_lines[sel.end.y];
                        let end_idx = get_utf_index(text_line, sel.end.x + 1);
                        out.queue(MoveTo(0, (sel_physical_y_end) as u16))?;
                        out.queue(Print(&text_line[..end_idx]))?;
                        out.queue(SetForegroundColor(Color::Reset))?;
                        out.queue(SetBackgroundColor(Color::Reset))?;
                    }
                }
            }
            None => {}
        }

        out.queue(MoveTo(self.cursor_x as u16, self.cursor_y as u16))?;
        out.flush()?;
        Ok(())
    }

    // TODO: Optimize to only redraw if there is scrolling. Else, just move the cursor
    fn scroll(&mut self, direction: ScrollDirection, amount: usize) -> io::Result<()> {
        match direction {
            ScrollDirection::Up => {
                if self.cursor_y <= SCROLLOFF && self.viewport_start >= amount {
                    self.viewport_start = self.viewport_start.saturating_sub(amount);
                    self.viewport_end = self.viewport_end.saturating_sub(amount);
                } else {
                    self.cursor_y = self.cursor_y.saturating_sub(amount);
                }
            }
            ScrollDirection::Down => {
                // BUG: sometimes it OOBs and crashes
                if self.cursor_y >= self.term_height - 1 - SCROLLOFF
                    && self.viewport_end < self.lines.len().saturating_sub(1)
                {
                    self.viewport_start = self.viewport_start.saturating_add(amount);
                    self.viewport_end = self.viewport_end.saturating_add(amount);
                } else if self.cursor_y < self.viewport_end.saturating_sub(1) {
                    self.cursor_y = self
                        .cursor_y
                        .saturating_add(amount)
                        .min(self.viewport_end - self.viewport_start);
                }
            }
        }
        if self.wish_cursor_x
            > self.text_lines[self.viewport_start + self.cursor_y]
                .chars()
                .count()
        {
            self.cursor_x = self.text_lines[self.viewport_start + self.cursor_y]
                .chars()
                .count()
                .saturating_sub(1);
        } else {
            self.cursor_x = self.wish_cursor_x;
        }
        self.draw()
    }

    fn move_sideways(&mut self, direction: SidewaysDirection, amount: usize) -> io::Result<()> {
        match direction {
            SidewaysDirection::Left => {
                self.cursor_x = self.cursor_x.saturating_sub(amount);
                self.wish_cursor_x = self.cursor_x;
            }
            SidewaysDirection::Right => {
                let line_length = self.text_lines[self.viewport_start + self.cursor_y]
                    .chars()
                    .count();
                if self.cursor_x <= line_length.saturating_sub(1) {
                    self.cursor_x = self.cursor_x.saturating_add(amount);
                    self.cursor_x = self.cursor_x.min(line_length);
                    self.wish_cursor_x = self.cursor_x;
                }
            }
        }
        execute!(stdout(), MoveTo(self.cursor_x as u16, self.cursor_y as u16))?;
        stdout().flush()?;
        Ok(())
    }

    pub fn handle_key_event(&mut self, event: KeyEvent) -> io::Result<()> {
        match event.code {
            crossterm::event::KeyCode::Char('j') => {
                self.scroll(ScrollDirection::Down, 1)?;
            }
            crossterm::event::KeyCode::Char('k') => {
                self.scroll(ScrollDirection::Up, 1)?;
            }
            crossterm::event::KeyCode::Char('d') => {
                self.scroll(ScrollDirection::Down, SCROLL_JUMP)?;
            }
            crossterm::event::KeyCode::Char('u') => {
                self.scroll(ScrollDirection::Up, SCROLL_JUMP)?;
            }
            crossterm::event::KeyCode::Char('h') => {
                self.move_sideways(SidewaysDirection::Left, 1)?;
            }
            crossterm::event::KeyCode::Char('l') => {
                self.move_sideways(SidewaysDirection::Right, 1)?;
            }

            // DEBUG: for moving faster, will be implemented with actual vim keys later
            crossterm::event::KeyCode::Char('b') => {
                self.move_sideways(SidewaysDirection::Left, SCROLL_JUMP)?;
            }
            crossterm::event::KeyCode::Char('w') => {
                self.move_sideways(SidewaysDirection::Right, SCROLL_JUMP)?;
            }
            _ => {}
        }
        Ok(())
    }
}

fn get_utf_index(line: &str, idx: usize) -> usize {
    line.char_indices()
        .nth(idx)
        .map(|(i, _)| i)
        .unwrap_or(line.len())
}

fn physical_to_logical_index(line: &str, mut physical: usize) -> (usize, usize) {
    const ESCAPE_CHAR: char = '\x1b';
    const RESET_RANGE: (char, char) = (64 as char, 126 as char);
    let mut escaped = false;
    let mut utf_offset = 0;
    for (i, c) in line.chars().enumerate() {
        if c == ESCAPE_CHAR {
            escaped = true;
        }
        if !escaped {
            if physical == 0 {
                return (i + utf_offset, utf_offset);
            }
            utf_offset += c.len_utf8() - 1;
            physical = physical.saturating_sub(1);
        }

        if c != '[' && c != ']' && c >= RESET_RANGE.0 && c <= RESET_RANGE.1 {
            escaped = false;
        }
    }
    (line.chars().count(), utf_offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_physical_to_logical_index() {
        let line = "Hello \x1b[31mWorld\x1b[0m!";
        let text_line = "Hello World!";
        for i in 0..text_line.chars().count() {
            assert_eq!(
                line.chars()
                    .nth(physical_to_logical_index(line, i).0)
                    .unwrap(),
                text_line.chars().nth(i).unwrap()
            );
        }
    }
}
