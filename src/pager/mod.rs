use crate::pager::search::{Search, SearchState};
use crate::selection::*;
use crate::settings::Settings;
use crossterm::event::{KeyCode, KeyEvent};
use std::collections::VecDeque;
use std::io::{self, stdin};
use strip_ansi_escapes::strip;

mod copy;
mod modes;
mod motions;
mod movement;
mod rendering;
mod search;

pub(crate) const INPUT_BUFFER_SIZE: usize = 4;

/// Represents a line of text, optimized for memory when no ANSI codes are present.
/// When a line contains ANSI escape codes, we store both the raw version (for copying)
/// and the stripped version (for display and logic).
/// When a line has no ANSI codes, we only store one String.
#[derive(Clone)]
pub enum Line {
    /// Line without ANSI escape codes - display and raw are the same
    Clean(String),
    /// Line with ANSI escape codes - need to store both
    Ansi { raw: String, stripped: String },
}

impl Line {
    /// Returns the display/stripped version of the line (without ANSI codes).
    /// This is used for rendering, cursor positioning, and search.
    pub fn display(&self) -> &str {
        match self {
            Line::Clean(s) => s,
            Line::Ansi { stripped, .. } => stripped,
        }
    }

    /// Returns the raw version of the line (with ANSI codes intact).
    /// This is used when copying text to preserve formatting.
    pub fn raw(&self) -> &str {
        match self {
            Line::Clean(s) => s,
            Line::Ansi { raw, .. } => raw,
        }
    }

    /// Returns the length of the display/stripped version.
    pub fn len(&self) -> usize {
        self.display().len()
    }

    /// Returns true if the display/stripped version is empty.
    pub fn is_empty(&self) -> bool {
        self.display().is_empty()
    }

    /// Creates a new Line from raw text, automatically detecting ANSI codes.
    pub fn new(raw: String) -> Self {
        let stripped_bytes = strip(raw.as_bytes());
        if stripped_bytes.len() == raw.len() && stripped_bytes == raw.as_bytes() {
            // No ANSI codes found, no stripping occurred
            Line::Clean(raw)
        } else {
            // ANSI codes were stripped, store both versions
            let stripped = String::from_utf8_lossy(&stripped_bytes).into_owned();
            Line::Ansi { raw, stripped }
        }
    }
}

pub struct Pager {
    pub(crate) lines: Vec<Line>,
    pub(crate) cursor_x: usize, // This is both the physical and logical position
    pub(crate) wish_cursor_x: usize,
    pub(crate) logical_y: usize,
    pub(crate) term_height: usize,
    pub(crate) term_width: usize,
    pub(crate) viewport_start: usize,
    pub(crate) viewport_end: usize,
    pub(crate) input_buffer: VecDeque<KeyCode>,
    pub(crate) selection: Option<Selection>, // We'll assume that start is always before end
    pub(crate) search: Option<Search>,
    pub(crate) settings: Settings,
    pub(crate) y_offset: usize, // Number of empty lines at the end of the file
}

// NOTE: Idea to implement the y_offset: Make the viewport support the fact that
// it spans beyond lines.len() - 1. viewport_end can be at most lines.len() - 1 + y_offset.
// When viewport_end is greater than lines.len() - 1, we cannot scroll down but only
// up. The rendering should take into account that it cannot use lines[viewport_end].
// However, we still have an offset of 1 compared to the normal rendering because
// the status line is always rendered at the bottom of the terminal. Plans are
// to move the status line to the top of the terminal, which means we will overlay
// it and only have to worry about the edge case where the pager is 1 line long
// in which (weird) case, we will shift it all down by 1 line and render the status
// line at the top as normal. Else, with the status line at the top, we can just
// render the pager as normal and not worry about the edge case at all.
// Maybe if I am feeling crazy (cray-cray) I can implement a cli flag to have
// the status line at the bottom or top of the terminal. Would suck to implement but
// really nice to have IMO.

impl Pager {
    pub fn new(mut settings: Settings) -> io::Result<Self> {
        let mut lines = Vec::<Line>::new();
        let tab_replacement = String::from(" ").repeat(settings.tab_width);
        for line in stdin().lines() {
            let mut line = line?;
            if line.contains('\t') {
                line = line.replace('\t', &tab_replacement);
            }
            // Create Line - will automatically detect ANSI and choose appropriate variant
            lines.push(Line::new(line));
        }
        let (term_width, term_height) = crossterm::terminal::size()?;
        settings.scroll_jump = term_height as usize / 2;

        // The pager may contain empty lines at the end
        let mut last_non_empty_line_idx = lines.len().saturating_sub(1);
        let mut y_offset = 0;
        while last_non_empty_line_idx > 0 && lines[last_non_empty_line_idx].display().is_empty() {
            last_non_empty_line_idx -= 1;
            y_offset += 1;
        }
        lines.truncate(last_non_empty_line_idx + 1);

        let cursor_x = lines
            .last()
            .map(|l| l.display().chars().count().saturating_sub(1))
            .unwrap_or(0)
            + settings.prompt_cursor_offset;

        Ok(Self {
            cursor_x,
            logical_y: lines.len().saturating_sub(1),

            y_offset,

            wish_cursor_x: cursor_x,

            term_height: term_height as usize,
            term_width: term_width as usize,

            viewport_start: lines
                .len()
                .saturating_sub((term_height as usize).saturating_sub(1)),
            viewport_end: lines.len().saturating_sub(1),

            lines,

            selection: None,
            search: None,
            input_buffer: VecDeque::with_capacity(INPUT_BUFFER_SIZE),
            settings,
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

    /// Gets the current line (display/stripped version)
    fn current_line(&self) -> &str {
        self.lines[self.logical_y].display()
    }

    /// Gets the current line length (display/stripped version)
    /// Warning: gets the utf-8 length
    fn current_line_len(&self) -> usize {
        self.current_line().chars().count()
    }
}
