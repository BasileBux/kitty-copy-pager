use crate::selection::*;
use crossterm::{
    QueueableCommand,
    clipboard::CopyToClipboard,
    cursor::MoveTo,
    event::{KeyCode, KeyEvent},
    execute,
    style::{Color, Print, SetBackgroundColor, SetForegroundColor},
    terminal::{Clear, ClearType},
};
use log::debug;
use std::collections::VecDeque;
use std::io::{self, Write, stdin, stdout};
use std::{cmp::min, io::Stdout};
use strip_ansi_escapes::strip;

const PROMPT_CURSOR_OFFSET: usize = 1;
const SCROLLOFF: usize = 4;
const SCROLL_JUMP: usize = 10;
const INPUT_BUFFER_SIZE: usize = 4;

enum VerticalDirection {
    Up,
    Down,
}

enum HorizontalDirection {
    Left,
    Right,
}

pub struct ScrollbackBuffer {
    lines: Vec<String>,
    text_lines: Vec<String>, // Lines without escape sequences
    cursor_x: usize,
    cursor_y: usize,
    wish_cursor_x: usize,
    term_height: usize,
    viewport_start: usize,
    viewport_end: usize,
    selection: Option<Selection>, // We'll assume that start is always before end
    input_buffer: VecDeque<KeyCode>,
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
        let (_, term_height) = crossterm::terminal::size()?;

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

            term_height: term_height as usize,

            viewport_start: raw_lines.len().saturating_sub(term_height as usize),
            viewport_end: raw_lines.len().saturating_sub(1),

            lines: raw_lines,
            text_lines,

            selection: None,
            input_buffer: VecDeque::with_capacity(INPUT_BUFFER_SIZE),
        })
    }

    pub fn handle_key_event(&mut self, event: KeyEvent) -> io::Result<bool> {
        match event.code {
            KeyCode::Char('q') => {
                return Ok(true);
            }
            KeyCode::Char('j') => {
                self.vertical_movement(VerticalDirection::Down, 1)?;
            }
            KeyCode::Char('k') => {
                self.vertical_movement(VerticalDirection::Up, 1)?;
            }
            KeyCode::Char('d') => {
                // Replaces ctrl+d
                self.vertical_movement(VerticalDirection::Down, SCROLL_JUMP)?;
            }
            KeyCode::Char('u') => {
                // Replaces ctrl+u
                self.vertical_movement(VerticalDirection::Up, SCROLL_JUMP)?;
            }
            KeyCode::Char('h') => {
                self.horizontal_movement(HorizontalDirection::Left, 1)?;
            }
            KeyCode::Char('l') => {
                self.horizontal_movement(HorizontalDirection::Right, 1)?;
            }

            KeyCode::Char('0') => {
                self.horizontal_movement(HorizontalDirection::Left, self.cursor_x)?;
            }
            KeyCode::Char('_') => {
                self.movement_underscore()?;
            }
            KeyCode::Char('$') => {
                let amount = self
                    .get_current_text_line_len()
                    .saturating_sub(self.cursor_x + 1);
                self.horizontal_movement(HorizontalDirection::Right, amount)?;
            }

            KeyCode::Char('w') => {
                self.movement_w(false)?;
            }
            KeyCode::Char('W') => {
                self.movement_w(true)?;
            }
            KeyCode::Char('b') => {
                self.movement_b(false)?;
            }
            KeyCode::Char('B') => {
                self.movement_b(true)?;
            }
            KeyCode::Char('e') => {
                self.movement_e(false)?;
            }
            KeyCode::Char('E') => {
                self.movement_e(true)?;
            }
            KeyCode::Char('G') => {
                self.movement_G()?;
            }
            KeyCode::Char('g') => {
                self.movement_gg()?;
                self.input_buffer.push_back(event.code);
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

            _ => {
                self.input_buffer.push_back(event.code);
            }
        }
        Ok(false)
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
                self.draw_selection(&sel, &mut out)?;
            }
            None => {}
        }

        out.queue(MoveTo(self.cursor_x as u16, self.cursor_y as u16))?;
        out.flush()?;
        Ok(())
    }

    fn draw_selection(&self, sel: &Selection, out: &mut Stdout) -> io::Result<()> {
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
        Ok(())
    }

    fn draw_cursor(&self) -> io::Result<()> {
        let mut out = stdout();
        out.queue(MoveTo(self.cursor_x as u16, self.cursor_y as u16))?;
        out.flush()?;
        Ok(())
    }

    // fn move_to(&mut self, x: usize, y: usize) {}
    // fn move_vertically_by(&mut self, dir: VerticalDirection, amount: usize) {}
    // fn move_horizontally_by(&mut self, dir: HorizontalDirection, amount: usize) {}

    fn vertical_movement(
        &mut self,
        direction: VerticalDirection,
        mut amount: usize,
    ) -> io::Result<()> {
        if amount == 0 {
            return Ok(());
        }
        let mut rerender = self.selection.is_some();
        match direction {
            VerticalDirection::Up => {
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
            VerticalDirection::Down => {
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

        let current_line_len = self.get_current_text_line_len();
        if self.wish_cursor_x > current_line_len {
            self.cursor_x = current_line_len.saturating_sub(1);
        } else {
            self.cursor_x = self.wish_cursor_x;
        }

        self.expand_selection();

        if rerender {
            self.draw()
        } else {
            self.draw_cursor()
        }
    }

    fn horizontal_movement(
        &mut self,
        direction: HorizontalDirection,
        amount: usize,
    ) -> io::Result<()> {
        if amount == 0 {
            return Ok(());
        }
        match direction {
            HorizontalDirection::Left => {
                self.cursor_x = self.cursor_x.saturating_sub(amount);
                self.wish_cursor_x = self.cursor_x;
            }
            HorizontalDirection::Right => {
                let line_length = self.get_current_text_line_len();
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
            self.draw_cursor()
        }
    }

    fn movement_underscore(&mut self) -> io::Result<()> {
        let mut amount = self.cursor_x;
        let mut jmp = 0;
        for (i, c) in self.get_current_text_line().chars().enumerate() {
            if !c.is_whitespace() {
                jmp = i;
                break;
            }
        }
        if self.cursor_x > jmp {
            amount = amount.saturating_sub(jmp);
            self.horizontal_movement(HorizontalDirection::Left, amount)
        } else {
            amount = jmp.saturating_sub(amount);
            self.horizontal_movement(HorizontalDirection::Right, amount)
        }
    }

    fn movement_w(&mut self, whitespace: bool) -> io::Result<()> {
        // TODO: Add wrapping when at end of line -> next line
        let line = self.get_current_text_line();
        let mut spaced = line
            .chars()
            .nth(get_utf_index(line, self.cursor_x))
            .unwrap_or('a')
            .is_ascii_punctuation();
        let mut amount = 0;
        let start = min(self.cursor_x + 1, line.len() - 1);
        for (i, c) in line[start..].chars().enumerate() {
            if (!whitespace && c.is_ascii_punctuation()) || (spaced && !c.is_whitespace()) {
                amount = i + 1;
                break;
            }
            if !spaced && c.is_whitespace() {
                spaced = true;
            }
        }
        self.horizontal_movement(HorizontalDirection::Right, amount)
    }

    fn movement_b(&mut self, whitespace: bool) -> io::Result<()> {
        // TODO: Add wrapping at start of line -> previous line
        let line = &self.get_current_text_line();
        let mut amount = 0;
        let end = get_utf_index(&line, self.cursor_x);
        for (i, c) in line[..end].chars().rev().enumerate() {
            let index = end.saturating_sub(i);
            let peek = line.chars().nth(index.saturating_sub(2)).unwrap_or('a');
            if (!c.is_whitespace()
                && (peek.is_whitespace()
                    || (peek.is_ascii_punctuation() && !whitespace)
                    || index <= 1))
                || (c.is_ascii_punctuation() && !whitespace)
            {
                amount = min(i + 1, end);
                break;
            }
        }
        self.horizontal_movement(HorizontalDirection::Left, amount)
    }

    fn movement_e(&mut self, whitespace: bool) -> io::Result<()> {
        // TODO: Add wrapping at end of line -> next line
        let line = &self.get_current_text_line();
        let start = get_utf_index(&line, self.cursor_x + 1);
        let line_end = &line[start..];
        let len = line_end.chars().enumerate().count().saturating_sub(1);
        let mut amount = 0;
        for (i, c) in line_end.chars().enumerate() {
            let peek = line_end.chars().nth(i + 1).unwrap_or('a');
            if (!c.is_whitespace()
                && (peek.is_whitespace() || (peek.is_ascii_punctuation() && !whitespace)))
                || (c.is_ascii_punctuation() && !whitespace)
                || i == len
            {
                amount = min(i + 1, len + 1);
                break;
            }
        }
        self.horizontal_movement(HorizontalDirection::Right, amount)
    }

    #[allow(non_snake_case)]
    fn movement_G(&mut self) -> io::Result<()> {
        self.vertical_movement(
            VerticalDirection::Down,
            self.lines.len() - self.get_cursor_logical_y(),
        )
    }

    // BUG: Only scrolls to the top of the viewport
    fn movement_gg(&mut self) -> io::Result<()> {
        if let Some(last) = self.input_buffer.iter().last()
            && last == &KeyCode::Char('g')
        {
            debug!(
                "y = {}, viewport_start = {}, viewport_end = {}, term_height = {}",
                self.get_cursor_logical_y(),
                self.viewport_start,
                self.viewport_end,
                self.term_height
            );
            self.vertical_movement(VerticalDirection::Up, self.get_cursor_logical_y())?;
        }
        Ok(())
    }

    fn get_cursor_logical_y(&self) -> usize {
        self.viewport_start + self.cursor_y
    }

    fn get_current_text_line(&self) -> &str {
        &self.lines[self.get_cursor_logical_y()]
    }

    /// Warning: gets the utf-8 length
    fn get_current_text_line_len(&self) -> usize {
        self.get_current_text_line().chars().count()
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
            let mut out = stdout();
            execute!(out, CopyToClipboard::to_clipboard_from(&copy_string))?;
            out.flush()?;
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
