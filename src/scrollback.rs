use crate::utils::get_utf_index;
use crate::{selection::*, utils::VimCharExt};
use crossterm::{
    QueueableCommand,
    clipboard::CopyToClipboard,
    cursor::MoveTo,
    event::{KeyCode, KeyEvent},
    execute,
    style::{Color, Print, SetBackgroundColor, SetForegroundColor},
    terminal::{Clear, ClearType},
};
use log::*;
use std::collections::VecDeque;
use std::io::{self, Write, stdin, stdout};
use std::{cmp::min, io::Stdout};
use strip_ansi_escapes::strip;

const PROMPT_CURSOR_OFFSET: usize = 1;
const SCROLLOFF: usize = 4;
const SCROLL_JUMP: usize = 10;
const INPUT_BUFFER_SIZE: usize = 4;

pub struct ScrollbackBuffer {
    lines: Vec<String>,
    text_lines: Vec<String>, // Lines without escape sequences
    cursor_x: usize,         // This is both the physical and logical position
    wish_cursor_x: usize,
    logical_y: usize,
    term_height: usize,
    viewport_start: usize,
    viewport_end: usize,
    input_buffer: VecDeque<KeyCode>,
    selection: Option<Selection>, // We'll assume that start is always before end
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

        Ok(Self {
            cursor_x,
            logical_y: raw_lines.len().saturating_sub(1),

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
            KeyCode::Char('q') => return Ok(true),

            // Simple movement
            KeyCode::Char('j') => self.move_vertically_by(1)?,
            KeyCode::Char('k') => self.move_vertically_by(-1)?,
            KeyCode::Char('d') => self.move_vertically_by(SCROLL_JUMP as isize)?, // Replaces ctrl+d
            KeyCode::Char('u') => self.move_vertically_by(-(SCROLL_JUMP as isize))?, // Replaces ctrl+u
            KeyCode::Char('h') => self.move_horizontally_by(-1)?,
            KeyCode::Char('l') => self.move_horizontally_by(1)?,

            // Movement to the end
            KeyCode::Char('0') => self.move_horizontally_to(0)?,
            KeyCode::Char('_') => self.movement_underscore()?,
            KeyCode::Char('$') => self.movement_dollar()?,

            // Movement to next/prev word
            KeyCode::Char('w') => self.movement_w(false)?,
            KeyCode::Char('W') => self.movement_w(true)?,
            KeyCode::Char('b') => self.movement_b(false)?,
            KeyCode::Char('B') => self.movement_b(true)?,
            KeyCode::Char('e') => self.movement_e(false)?,
            KeyCode::Char('E') => self.movement_e(true)?,

            // Top/Bottom movement
            KeyCode::Char('G') => self.movement_G()?,
            KeyCode::Char('g') => {
                if self.movement_gg()? {
                    self.movement_suffix(true)?;
                }
                self.input_buffer.push_back(event.code);
            }

            // Selection motions
            KeyCode::Char('v') => {
                self.selection = Some(Selection::with_coords(
                    self.cursor_x,
                    self.logical_y,
                    self.cursor_x,
                    self.logical_y,
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
        for (i, line) in self.lines[self.viewport_start..self.viewport_end.saturating_add(1)]
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

        out.queue(MoveTo(
            self.cursor_x as u16,
            self.get_physical_cursor_y() as u16,
        ))?;
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
        out.queue(MoveTo(
            self.cursor_x as u16,
            self.get_physical_cursor_y() as u16,
        ))?;
        if let Some(sel) = &self.selection {
            self.draw_selection(&sel, &mut out)?;
        }
        out.flush()?;
        Ok(())
    }

    fn get_physical_cursor_y(&self) -> usize {
        self.logical_y.saturating_sub(self.viewport_start)
    }

    fn move_to(&mut self, x: usize, y: usize) -> io::Result<()> {
        self.logical_y = min(y, self.lines.len().saturating_sub(1));
        let line_len = self.current_line_len();
        self.cursor_x = min(x, line_len.saturating_sub(1));
        self.movement_suffix(true)
    }

    // BUG: should execute movement function on next line and not just go to index 0
    fn wrap_to_next(&mut self) -> io::Result<()> {
        let mut wrapped = false;
        let y = self.logical_y.saturating_add(1);
        if y <= self.lines.len().saturating_sub(1) {
            self.logical_y = y;
            self.cursor_x = 0;
            wrapped = true;
        }
        self.movement_suffix(wrapped)
    }

    // BUG: should execute movement function on previous line and not just go to end of line
    fn wrap_to_previous(&mut self) -> io::Result<()> {
        let mut wrapped = false;
        let y = self.logical_y.saturating_sub(1);
        if y > 0 {
            self.logical_y = y;
            self.cursor_x = self.current_line_len().saturating_sub(1);
            wrapped = true;
        }
        self.movement_suffix(wrapped)
    }

    fn move_vertically_by(&mut self, amount: isize) -> io::Result<()> {
        self.logical_y = min(
            self.logical_y.saturating_add_signed(amount),
            self.lines.len().saturating_sub(1),
        );
        let line_len = self.current_line_len();
        self.cursor_x = min(self.wish_cursor_x, line_len.saturating_sub(1));
        self.movement_suffix(true)
    }

    fn move_vertically_to(&mut self, y: usize) -> io::Result<()> {
        self.move_to(self.cursor_x, y)?;
        let line_len = self.current_line_len();
        self.cursor_x = min(self.wish_cursor_x, line_len.saturating_sub(1));
        self.movement_suffix(true)
    }

    fn move_horizontally_by(&mut self, amount: isize) -> io::Result<()> {
        let line_len = self.current_line_len();
        self.cursor_x = min(
            self.cursor_x.saturating_add_signed(amount),
            line_len.saturating_sub(1),
        );
        self.wish_cursor_x = self.cursor_x;
        self.movement_suffix(false)
    }

    fn move_horizontally_to(&mut self, x: usize) -> io::Result<()> {
        self.move_to(x, self.logical_y)?;
        self.wish_cursor_x = self.cursor_x;
        self.movement_suffix(false)
    }

    fn move_viewport(&mut self) -> bool {
        let upper_bound = self.logical_y.saturating_add(SCROLLOFF);
        if upper_bound > self.viewport_end {
            let mut movement = self.viewport_end;
            self.viewport_end = min(
                self.logical_y.saturating_add(SCROLLOFF),
                self.lines.len().saturating_sub(1),
            );
            movement = self.viewport_end - movement;
            self.viewport_start = self.viewport_start.saturating_add(movement);
            return true;
        }
        if self.logical_y.saturating_sub(SCROLLOFF) < self.viewport_start {
            let mut movement = self.viewport_start;
            self.viewport_start = self.logical_y.saturating_sub(SCROLLOFF);
            movement = movement - self.viewport_start;
            self.viewport_end = self.viewport_end.saturating_sub(movement);
            return true;
        }
        false
    }

    fn movement_suffix(&mut self, rerender: bool) -> io::Result<()> {
        if (rerender && self.move_viewport()) || self.selection.is_some() {
            self.expand_selection();
            self.draw()
        } else {
            self.draw_cursor()
        }
    }

    fn movement_underscore(&mut self) -> io::Result<()> {
        let mut jmp = 0;
        for (i, c) in self.current_line().chars().enumerate() {
            if !c.is_whitespace() {
                jmp = i;
                break;
            }
        }
        self.move_horizontally_to(jmp)
    }

    fn movement_w(&mut self, whitespace: bool) -> io::Result<()> {
        let line = self.current_line();
        let mut spaced = line
            .chars()
            .nth(self.cursor_x)
            .unwrap_or('a')
            .is_vim_punctuation();
        let mut amount = 0;
        let start = get_utf_index(&line, self.cursor_x.saturating_add(1));
        for (i, c) in line[start..].chars().enumerate() {
            let prev_is_punctuation = line
                .chars()
                .nth(self.cursor_x.saturating_add(i))
                .unwrap_or('a')
                .is_vim_punctuation();
            if (!whitespace && c.is_vim_punctuation() && !prev_is_punctuation)
                || (spaced && !c.is_whitespace() && !c.is_vim_punctuation())
            {
                amount = i.saturating_add(1);
                break;
            }
            if !spaced && c.is_whitespace() {
                spaced = true;
            }
        }
        if amount != 0 {
            self.move_horizontally_by(amount as isize)
        } else {
            self.wrap_to_next()
        }
    }

    // BUG: prevalent to all motions under. When multiple punctuations are contiguous,
    // it won't skip them, it will go over each individually. Example: "-------"
    fn movement_b(&mut self, whitespace: bool) -> io::Result<()> {
        let line = &self.current_line();
        let mut amount = 0;
        let end = get_utf_index(&line, self.cursor_x);
        if self.cursor_x == 0 {
            return self.wrap_to_previous();
        }
        for (i, c) in line[..end].chars().rev().enumerate() {
            let index = end.saturating_sub(i);
            let peek = line.chars().nth(index.saturating_sub(2)).unwrap_or('a');
            if (!c.is_whitespace()
                && (peek.is_whitespace()
                    || (peek.is_vim_punctuation() && !whitespace)
                    || index <= 1))
                || (c.is_vim_punctuation() && !whitespace)
            {
                amount = min(i + 1, end);
                break;
            }
        }
        self.move_horizontally_by(-(amount as isize))
    }

    fn movement_e(&mut self, whitespace: bool) -> io::Result<()> {
        let line = &self.current_line();
        let start = get_utf_index(&line, self.cursor_x.saturating_add(1));
        let line_end = &line[start..];
        let mut len = line_end.chars().enumerate().count();
        if len <= 0 {
            return self.wrap_to_next();
        }
        len = len.saturating_sub(1);
        for (i, c) in line_end.chars().enumerate() {
            let peek = line_end.chars().nth(i + 1).unwrap_or('a');
            if (!c.is_whitespace()
                && (peek.is_whitespace() || (peek.is_vim_punctuation() && !whitespace)))
                || (c.is_vim_punctuation() && !whitespace)
                || i == len
            {
                return self.move_horizontally_by(min(i + 1, len + 1) as isize);
            }
        }
        Ok(())
    }

    fn movement_dollar(&mut self) -> io::Result<()> {
        self.move_horizontally_to(self.current_line_len().saturating_sub(1))
    }

    #[allow(non_snake_case)]
    fn movement_G(&mut self) -> io::Result<()> {
        self.move_to(self.wish_cursor_x, self.lines.len().saturating_sub(1))
    }

    fn movement_gg(&mut self) -> io::Result<bool> {
        if let Some(last) = self.input_buffer.iter().last()
            && last == &KeyCode::Char('g')
        {
            self.move_to(self.wish_cursor_x, 0)?;
            return Ok(true);
        }
        return Ok(false);
    }

    /// Gets the current text_line
    fn current_line(&self) -> &str {
        &self.text_lines[self.logical_y]
    }

    /// Gets the current text_line
    /// Warning: gets the utf-8 length
    fn current_line_len(&self) -> usize {
        self.current_line().chars().count()
    }

    fn expand_selection(&mut self) {
        let y = self.logical_y;
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
