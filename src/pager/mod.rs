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

pub struct Pager {
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
        let mut raw_lines = Vec::<String>::new();
        let mut text_lines = Vec::<String>::new();
        let tab_replacement = String::from(" ").repeat(settings.tab_width);
        for line in stdin().lines() {
            let mut line = line?;
            if line.contains('\t') {
                line = line.replace('\t', &tab_replacement);
            }
            let stripped_bytes = strip(line.as_bytes());
            let stripped = if stripped_bytes.len() == line.len() && stripped_bytes == line.as_bytes() {
                line.clone()
            } else {
                String::from_utf8_lossy(&stripped_bytes).into_owned()
            };
            text_lines.push(stripped);
            raw_lines.push(line);
        }
        let (term_width, term_height) = crossterm::terminal::size()?;
        settings.scroll_jump = term_height as usize / 2;

        // The pager may contain empty lines at the end
        let mut last_non_empty_line_idx = raw_lines.len().saturating_sub(1);
        let mut y_offset = 0;
        while last_non_empty_line_idx > 0 && raw_lines[last_non_empty_line_idx].is_empty() {
            last_non_empty_line_idx -= 1;
            y_offset += 1;
        }
        raw_lines.truncate(last_non_empty_line_idx + 1);
        text_lines.truncate(last_non_empty_line_idx + 1);

        let cursor_x = text_lines
            .last()
            .map(|l| l.chars().count().saturating_sub(1))
            .unwrap_or(0)
            + settings.prompt_cursor_offset;

        Ok(Self {
            cursor_x,
            logical_y: raw_lines.len().saturating_sub(1),

            y_offset,

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
