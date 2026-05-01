use crate::scrollback::search::{Search, SearchState};
use crate::selection::*;
use crossterm::event::{KeyCode, KeyEvent};
use crossterm::style::Color;
use std::collections::VecDeque;
use std::io::{self, stdin};
use strip_ansi_escapes::strip;

mod copy;
mod modes;
mod motions;
mod movement;
mod rendering;
mod search;

pub(crate) const PROMPT_CURSOR_OFFSET: usize = 1;
pub(crate) const SCROLLOFF: usize = 4;
pub(crate) const SCROLL_JUMP: usize = 10;
pub(crate) const INPUT_BUFFER_SIZE: usize = 4;
pub(crate) const TAB_WIDTH: usize = 8;

pub(crate) const REAL_TIME_SEARCH: bool = false;
pub(crate) const SMARTCASE_SEARCH: bool = true;

pub(crate) const STATUS_LINE_BG_COLOR: Color = Color::DarkGrey;
pub(crate) const STATUS_LINE_FG_COLOR: Color = Color::White;

pub(crate) const SEARCH_ERROR_FG_COLOR: Color = Color::Red;

pub(crate) const SELECTION_BG_COLOR: Color = Color::Yellow;
pub(crate) const SELECTION_FG_COLOR: Color = Color::Black;

pub(crate) const SEARCH_HIGHLIGHT_BG_COLOR: Color = Color::Blue;
pub(crate) const SEARCH_HIGHLIGHT_FG_COLOR: Color = Color::Black;

pub struct ScrollbackBuffer {
    pub(crate) lines: Vec<String>,
    pub(crate) text_lines: Vec<String>, // Lines without escape sequences
    pub(crate) cursor_x: usize,         // This is both the physical and logical position
    pub(crate) wish_cursor_x: usize,
    pub(crate) logical_y: usize,
    pub(crate) term_height: usize,
    pub(crate) term_width: usize,
    pub(crate) viewport_start: usize,
    pub(crate) viewport_end: usize,
    pub(crate) input_buffer: VecDeque<KeyCode>,
    pub(crate) selection: Option<Selection>, // We'll assume that start is always before end
    pub(crate) search: Option<Search>,
}

impl ScrollbackBuffer {
    pub fn new() -> io::Result<Self> {
        let mut raw_lines = Vec::<String>::new();
        let mut text_lines = Vec::<String>::new();
        let tab_replacement = String::from(" ").repeat(TAB_WIDTH);
        for line in stdin().lines() {
            let mut line = line?;
            line = line.replace("\t", &tab_replacement);
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

        Ok(Self {
            cursor_x,
            logical_y: raw_lines.len().saturating_sub(1),

            wish_cursor_x: cursor_x,

            term_height: term_height as usize,
            term_width: term_width as usize,

            viewport_start: raw_lines
                .len()
                .saturating_sub((term_height as usize).saturating_sub(1)),
            viewport_end: raw_lines.len().saturating_sub(1),

            lines: raw_lines,
            text_lines,

            selection: None,
            search: None,
            input_buffer: VecDeque::with_capacity(INPUT_BUFFER_SIZE),
        })
    }

    pub fn handle_key_event(&mut self, event: KeyEvent) -> io::Result<bool> {
        if let Some(search) = &self.search
            && search.state == SearchState::Typing
        {
            let exec_search = self.search_mode(event)?;

            if exec_search {
                self.search();
                if let Some(search) = &self.search
                    && search.error.is_some()
                {
                    self.draw()?;
                }
                self.move_to_closest_next_match()?;
            }
            Ok(false)
        } else {
            self.normal_mode(event)
        }
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
}
