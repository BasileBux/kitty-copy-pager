use crate::selection::*;
use crossterm::event::{KeyCode, KeyEvent};
use std::collections::VecDeque;
use std::io::{self, stdin};
use strip_ansi_escapes::strip;

mod copy;
mod motions;
mod movement;
mod rendering;

const PROMPT_CURSOR_OFFSET: usize = 1;
const SCROLLOFF: usize = 4;
const SCROLL_JUMP: usize = 10;
const INPUT_BUFFER_SIZE: usize = 4;
const TAB_WIDTH: usize = 8;

pub struct ScrollbackBuffer {
    pub(crate) lines: Vec<String>,
    pub(crate) text_lines: Vec<String>, // Lines without escape sequences
    pub(crate) cursor_x: usize,         // This is both the physical and logical position
    pub(crate) wish_cursor_x: usize,
    pub(crate) logical_y: usize,
    pub(crate) term_height: usize,
    pub(crate) viewport_start: usize,
    pub(crate) viewport_end: usize,
    pub(crate) input_buffer: VecDeque<KeyCode>,
    pub(crate) selection: Option<Selection>, // We'll assume that start is always before end
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
            KeyCode::Char('_') | KeyCode::Char('^') => self.movement_underscore()?,
            KeyCode::Char('$') => self.movement_dollar()?,

            // Movement to next/prev word
            KeyCode::Char('w') => self.movement_w(false, false)?,
            KeyCode::Char('W') => self.movement_w(true, false)?,
            KeyCode::Char('b') => self.movement_b(false, false)?,
            KeyCode::Char('B') => self.movement_b(true, false)?,
            KeyCode::Char('e') => self.movement_e(false, false)?,
            KeyCode::Char('E') => self.movement_e(true, false)?,

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
