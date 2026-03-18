use crossterm::{
    QueueableCommand,
    clipboard::CopyToClipboard,
    cursor::MoveTo,
    event::{KeyCode, KeyEvent},
    execute,
    style::{Color, Print, SetBackgroundColor, SetForegroundColor},
    terminal::{Clear, ClearType},
};
use std::cmp::{PartialEq, min};
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

#[derive(Clone)]
pub struct Vec2<T> {
    x: T,
    y: T,
}
impl<T> Vec2<T> {
    pub fn new(x: T, y: T) -> Self {
        Vec2 { x, y }
    }
}

impl<T: PartialEq> PartialEq for Vec2<T> {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y
    }
}

#[derive(Clone)]
enum SelectedEnd {
    Start,
    End,
}

#[derive(Clone)]
pub struct Selection {
    start: Vec2<usize>,
    end: Vec2<usize>,
    sel_end: SelectedEnd,
}

impl Selection {
    pub fn new(start: Vec2<usize>, end: Vec2<usize>) -> Self {
        Selection {
            start,
            end,
            sel_end: SelectedEnd::End,
        }
    }

    pub fn with_coords(start_x: usize, start_y: usize, end_x: usize, end_y: usize) -> Self {
        Selection {
            start: Vec2::new(start_x, start_y),
            end: Vec2::new(end_x, end_y),
            sel_end: SelectedEnd::End,
        }
    }

    pub fn swap_ends_to(&mut self, to_x: usize, to_y: usize) {
        match self.sel_end {
            SelectedEnd::Start => {
                self.start = self.end.clone();
                self.end = Vec2::new(to_x, to_y);
            }
            SelectedEnd::End => {
                self.end = self.start.clone();
                self.start = Vec2::new(to_x, to_y);
            }
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

            selection: None,
        })
    }

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
                    let sel_physical_y_end = sel.end.y - self.viewport_start;

                    if sel_physical_y_start < 0 {
                        out.queue(MoveTo(0, 0))?;
                    } else {
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

                        let loop_start = if sel_physical_y_start < 0 {
                            sel.start.y + sel_physical_y_start.wrapping_abs() as usize
                        } else {
                            sel.start.y + 1
                        };

                        let loop_end = if sel_physical_y_end < self.term_height {
                            sel.end.y
                        } else {
                            sel.end.y - (sel_physical_y_end - self.term_height)
                        };

                        for (i, line) in self.text_lines[loop_start..loop_end].iter().enumerate() {
                            out.queue(MoveTo(0, (y_idx + i) as u16))?;
                            out.queue(Print(line))?;
                        }
                        if sel_physical_y_end < self.term_height {
                            let text_line = &self.text_lines[sel.end.y];
                            let end_idx = get_utf_index(text_line, sel.end.x + 1);
                            out.queue(MoveTo(0, (sel_physical_y_end) as u16))?;
                            out.queue(Print(&text_line[..end_idx]))?;
                        }
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

    fn render_cursor(&self) -> io::Result<()> {
        stdout().queue(MoveTo(self.cursor_x as u16, self.cursor_y as u16))?;
        stdout().flush()?;
        Ok(())
    }

    fn scroll(&mut self, direction: ScrollDirection, mut amount: usize) -> io::Result<()> {
        let mut rerender = self.selection.is_some();
        match direction {
            ScrollDirection::Up => {
                if self.cursor_y <= SCROLLOFF && self.viewport_start >= 1 {
                    amount = if self.viewport_start >= amount {
                        amount
                    } else {
                        self.viewport_start
                    };

                    self.viewport_start = self.viewport_start.saturating_sub(amount);
                    self.viewport_end = self.viewport_end.saturating_sub(amount);
                    rerender = true;
                } else {
                    self.cursor_y = self.cursor_y.saturating_sub(amount);
                }
            }
            ScrollDirection::Down => {
                let max_len = self.lines.len().saturating_sub(1);
                if self.cursor_y >= self.term_height - 1 - SCROLLOFF && self.viewport_end < max_len
                {
                    amount = if self.viewport_end < self.lines.len().saturating_sub(amount) {
                        amount
                    } else {
                        max_len.saturating_sub(self.viewport_end)
                    };
                    self.viewport_start = self.viewport_start.saturating_add(amount);
                    self.viewport_end = self.viewport_end.saturating_add(amount);
                    rerender = true;
                } else if self.cursor_y < self.viewport_end {
                    self.cursor_y = self
                        .cursor_y
                        .saturating_add(amount)
                        .min(self.viewport_end - self.viewport_start);
                }
            }
        }

        if self.wish_cursor_x > self.text_lines[self.get_cursor_logical_y()].chars().count() {
            self.cursor_x = self.text_lines[self.get_cursor_logical_y()]
                .chars()
                .count()
                .saturating_sub(1);
        } else {
            self.cursor_x = self.wish_cursor_x;
        }

        self.expand_selection();

        if rerender {
            self.draw()
        } else {
            self.render_cursor()
        }
    }

    fn move_sideways(&mut self, direction: SidewaysDirection, amount: usize) -> io::Result<()> {
        match direction {
            SidewaysDirection::Left => {
                self.cursor_x = self.cursor_x.saturating_sub(amount);
                self.wish_cursor_x = self.cursor_x;
            }
            SidewaysDirection::Right => {
                let line_length = self.text_lines[self.get_cursor_logical_y()].chars().count();
                if self.cursor_x <= line_length.saturating_sub(1) {
                    self.cursor_x = self.cursor_x.saturating_add(amount);
                    self.cursor_x = self.cursor_x.min(line_length);
                    self.wish_cursor_x = self.cursor_x;
                }
            }
        }
        self.expand_selection();
        if self.selection.is_some() {
            self.draw()
        } else {
            self.render_cursor()
        }
    }

    fn get_cursor_logical_y(&self) -> usize {
        self.viewport_start + self.cursor_y
    }

    fn expand_selection(&mut self) {
        let y = self.get_cursor_logical_y();
        if let Some(sel) = &mut self.selection {
            match sel.sel_end {
                SelectedEnd::Start => {
                    if y > sel.end.y || (y == sel.end.y && self.cursor_x > sel.end.x) {
                        sel.swap_ends_to(self.cursor_x, y);
                        sel.sel_end = SelectedEnd::End;
                    } else {
                        sel.start = Vec2::new(self.cursor_x, y);
                    }
                }
                SelectedEnd::End => {
                    if y < sel.start.y || (y == sel.start.y && self.cursor_x < sel.start.x) {
                        sel.swap_ends_to(self.cursor_x, y);
                        sel.sel_end = SelectedEnd::Start;
                    } else {
                        sel.end = Vec2::new(self.cursor_x, y);
                    }
                }
            }
        }
    }

    fn copy_selection(&self) -> io::Result<()> {
        if let Some(sel) = &self.selection {
            let mut copy_string = String::new();
            let end_y = sel.end.y.min(self.text_lines.len().saturating_sub(1));
            let last_i = end_y - sel.start.y;

            for (i, line) in self.text_lines[sel.start.y..=end_y].iter().enumerate() {
                if i == 0 && i == last_i {
                    let start = get_utf_index(line, sel.start.x);
                    let end = min(get_utf_index(line, sel.end.x), line.len().saturating_sub(1));
                    copy_string.push_str(&line[start..end + 1]);
                } else if i == 0 {
                    let start = get_utf_index(line, sel.start.x);
                    copy_string.push_str(&line[start..]);
                    copy_string.push_str("\n");
                } else if i == last_i {
                    let end = min(get_utf_index(line, sel.end.x), line.len().saturating_sub(2));
                    copy_string.push_str(&line[..end + 1]);
                    copy_string.push_str("\n");
                } else {
                    copy_string.push_str(line);
                    copy_string.push_str("\n");
                }
            }
            execute!(stdout(), CopyToClipboard::to_clipboard_from(&copy_string))?;
            stdout().flush()?;
        }
        Ok(())
    }

    pub fn handle_key_event(&mut self, event: KeyEvent) -> io::Result<bool> {
        match event.code {
            KeyCode::Char('q') => {
                return Ok(true);
            }
            KeyCode::Char('j') => {
                self.scroll(ScrollDirection::Down, 1)?;
            }
            KeyCode::Char('k') => {
                self.scroll(ScrollDirection::Up, 1)?;
            }
            KeyCode::Char('d') => {
                self.scroll(ScrollDirection::Down, SCROLL_JUMP)?;
            }
            KeyCode::Char('u') => {
                self.scroll(ScrollDirection::Up, SCROLL_JUMP)?;
            }
            KeyCode::Char('h') => {
                self.move_sideways(SidewaysDirection::Left, 1)?;
            }
            KeyCode::Char('l') => {
                self.move_sideways(SidewaysDirection::Right, 1)?;
            }

            KeyCode::Char('v') => {
                self.selection = Some(Selection::with_coords(
                    self.cursor_x,
                    self.get_cursor_logical_y(),
                    self.cursor_x,
                    self.get_cursor_logical_y(),
                ));
            }
            KeyCode::Char('y') | KeyCode::Enter => {
                self.copy_selection()?;
                return Ok(true);
            }
            KeyCode::Esc => {
                self.selection = None;
                self.draw()?;
            }

            // DEBUG: for moving faster, will be implemented with actual vim keys later
            // TODO: implement correct movement
            crossterm::event::KeyCode::Char('b') => {
                self.move_sideways(SidewaysDirection::Left, SCROLL_JUMP)?;
            }
            crossterm::event::KeyCode::Char('w') => {
                self.move_sideways(SidewaysDirection::Right, SCROLL_JUMP)?;
            }
            _ => {}
        }
        Ok(false)
    }
}

fn get_utf_index(line: &str, idx: usize) -> usize {
    line.char_indices()
        .nth(idx)
        .map(|(i, _)| i)
        .unwrap_or(line.len())
}
