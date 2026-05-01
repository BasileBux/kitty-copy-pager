use super::ScrollbackBuffer;

use crate::scrollback::search::SearchState;
use crate::scrollback::{
    REAL_TIME_SEARCH, SEARCH_ERROR_FG_COLOR, SEARCH_HIGHLIGHT_BG_COLOR, SEARCH_HIGHLIGHT_FG_COLOR,
    SELECTION_BG_COLOR, SELECTION_FG_COLOR, STATUS_LINE_BG_COLOR, STATUS_LINE_FG_COLOR,
};
use crate::selection::*;
use crate::utils::get_utf_index;
use crossterm::{
    QueueableCommand,
    cursor::MoveTo,
    style::{Attribute, Color, Print, SetAttribute, SetBackgroundColor, SetForegroundColor},
    terminal::{Clear, ClearType},
};
use std::io::{self, Stdout, Write, stdout};
use unicode_width::UnicodeWidthStr;

const PROMPT_ELIPSIS: &str = "... (truncated)";
const STATUS_LINE_AVG_LEN: usize = "Ln 123, Col 123".len();

impl ScrollbackBuffer {
    pub fn draw_status_line(&mut self) -> io::Result<()> {
        let mut out = stdout();
        out.queue(MoveTo(0, self.term_height as u16))?;
        out.queue(SetBackgroundColor(STATUS_LINE_BG_COLOR))?;
        out.queue(SetForegroundColor(STATUS_LINE_FG_COLOR))?;
        out.queue(Clear(ClearType::CurrentLine))?;

        let mut cursor_x = self.get_physical_cursor_x() as u16;
        let mut cursor_y = self.get_physical_cursor_y() as u16;

        let status_text = format!("Ln {}, Col {}", self.logical_y, self.cursor_x);

        let mut long_search = false;
        let mut search_line_number = 1;
        if let Some(search) = &mut self.search {
            let search_query_width = search.query.width();
            let long_search_threshold = self.term_width as usize - (status_text.width() * 2);
            if search_query_width > long_search_threshold {
                search_line_number = (search_query_width as f64
                    / self.term_width.saturating_sub(1) as f64)
                    .ceil() as usize;
                if search_line_number > (self.term_height as usize / 2) {
                    search.error = Some("Search query too long to display".to_string());
                } else {
                    long_search = true;
                }
            }

            out.queue(MoveTo(0, self.term_height as u16))?;
            if let Some(error) = &search.error {
                out.queue(SetForegroundColor(SEARCH_ERROR_FG_COLOR))?;
                out.queue(Print(error.as_str()))?;
                out.queue(SetForegroundColor(STATUS_LINE_FG_COLOR))?;
            } else {
                let mut search_prompt = search.query.as_str();
                if search.state == SearchState::Typing {
                    if long_search {
                        cursor_x = (search.query.width() % self.term_width) as u16 + 1;
                        out.queue(MoveTo(
                            0,
                            (self.term_height as u16).saturating_sub(search_line_number as u16),
                        ))?;
                    } else {
                        cursor_x = search.query.width() as u16 + 1;
                    }
                    cursor_y = self.term_height as u16;
                } else if long_search {
                    search_prompt = &search.query[..get_utf_index(
                        &search.query,
                        (self.term_width as usize)
                            .saturating_sub(STATUS_LINE_AVG_LEN * 2 + PROMPT_ELIPSIS.width() + 1),
                    )];
                }
                // BUG: when `REAL_TIME_SEARCH = true`, cursor is highjacked and moves
                // to the last match, which should not happen
                if search.state != SearchState::Hidden {
                    out.queue(Print(&format!("/{}", search_prompt)))?;
                    if long_search && search.state != SearchState::Typing {
                        out.queue(Print(PROMPT_ELIPSIS))?;
                    }
                }
            }
            search.long_search = long_search;
        }

        if !long_search {
            out.queue(MoveTo(
                self.term_width.saturating_sub(status_text.width()) as u16,
                self.term_height as u16,
            ))?;
            out.queue(Print(status_text))?;
        }

        out.queue(MoveTo(cursor_x, cursor_y))?;
        out.flush()
    }

    pub fn draw(&mut self) -> io::Result<()> {
        let mut out = stdout();

        self.draw_text(&mut out)?;
        self.draw_selection(&mut out)?;
        self.draw_search(true)?;
        self.draw_status_line()?;

        out.flush()
    }

    pub(crate) fn draw_text(&self, out: &mut Stdout) -> io::Result<()> {
        out.queue(SetBackgroundColor(Color::Reset))?;
        out.queue(SetForegroundColor(Color::Reset))?;
        out.queue(Clear(ClearType::All))?;
        for (i, line) in self.lines[self.viewport_start..self.viewport_end.saturating_add(1)]
            .iter()
            .enumerate()
        {
            out.queue(MoveTo(0, i as u16))?;
            out.queue(Print(line))?;
        }
        Ok(())
    }

    pub(crate) fn draw_highlight(
        &self,
        start: &Vec2<usize>,
        end: &Vec2<usize>,
        fg_color: &Color,
        bg_color: &Color,
        out: &mut Stdout,
    ) -> io::Result<()> {
        if end.y < self.viewport_start || start.y > self.viewport_end {
            return Ok(());
        }

        let sel_physical_y_start = start.y as isize - self.viewport_start as isize;
        let sel_physical_y_end = end.y - self.viewport_start;

        if sel_physical_y_start < 0 {
            out.queue(MoveTo(0, 0))?;
        } else {
            out.queue(MoveTo(start.x as u16, sel_physical_y_start as u16))?;
        }

        out.queue(SetAttribute(Attribute::Reset))?;
        out.queue(SetForegroundColor(*fg_color))?;
        out.queue(SetBackgroundColor(*bg_color))?;

        if start.y == end.y {
            // Single-line selection
            if start.y < self.text_lines.len() {
                let text_line = &self.text_lines[start.y];
                let start_idx = get_utf_index(text_line, start.x);
                let end_idx = get_utf_index(text_line, end.x + 1);
                out.queue(Print(&text_line[start_idx..end_idx]))?;
            }
        } else {
            // Multi-line selection
            let y_idx = if sel_physical_y_start < 0 {
                0usize
            } else {
                (sel_physical_y_start as usize).saturating_add(1)
            };

            // Draw the first line (partial)
            if sel_physical_y_start >= 0 && start.y < self.text_lines.len() {
                let text_line = &self.text_lines[start.y];
                let start_idx = get_utf_index(text_line, start.x);
                out.queue(Print(&text_line[start_idx..]))?;
            }

            let loop_start = if sel_physical_y_start < 0 {
                start.y + sel_physical_y_start.wrapping_abs() as usize
            } else {
                start.y + 1
            };

            let loop_end = if sel_physical_y_end < self.term_height {
                end.y
            } else {
                end.y - (sel_physical_y_end - self.term_height)
            }
            .min(self.text_lines.len());

            for (i, line) in self.text_lines[loop_start..loop_end].iter().enumerate() {
                out.queue(MoveTo(0, (y_idx + i) as u16))?;
                if line.is_empty() {
                    // Render a space on empty lines to keep selection continuous
                    out.queue(Print(" "))?;
                } else {
                    out.queue(Print(line))?;
                }
            }

            // Draw the last line (partial)
            if sel_physical_y_end < self.term_height && end.y < self.text_lines.len() {
                let text_line = &self.text_lines[end.y];
                let end_idx = get_utf_index(text_line, end.x + 1);
                out.queue(MoveTo(0, sel_physical_y_end as u16))?;
                out.queue(Print(&text_line[..end_idx]))?;
            }
        }

        out.queue(SetForegroundColor(Color::Reset))?;
        out.queue(SetBackgroundColor(Color::Reset))?;
        Ok(())
    }

    pub(crate) fn draw_cursor(&self) -> io::Result<()> {
        let mut out = stdout();
        out.queue(MoveTo(
            self.get_physical_cursor_x() as u16,
            self.get_physical_cursor_y() as u16,
        ))?;
        self.draw_selection(&mut out)?;
        out.flush()
    }

    fn get_physical_cursor_x(&self) -> usize {
        self.current_line()[..get_utf_index(self.current_line(), self.cursor_x)].width()
    }

    pub(crate) fn get_physical_cursor_y(&self) -> usize {
        self.logical_y.saturating_sub(self.viewport_start)
    }

    pub(crate) fn draw_selection(&self, out: &mut Stdout) -> io::Result<()> {
        if let Some(sel) = &self.selection {
            self.draw_highlight(
                &sel.start,
                &sel.end,
                &SELECTION_FG_COLOR,
                &SELECTION_BG_COLOR,
                out,
            )?;
        }
        Ok(())
    }

    pub(crate) fn draw_search(&mut self, force: bool) -> io::Result<()> {
        let mut search = match self.search.take() {
            Some(s) => s,
            None => return Ok(()),
        };

        // Draw highlights when:
        // - PendingRedraw (search just executed)
        // - Highlighted (already drawn before, force redraw)
        // - Typing (real-time search active)
        if (search.state == SearchState::PendingRedraw
            || (search.state == SearchState::Typing && REAL_TIME_SEARCH)
            || (force && search.state == SearchState::Highlighted))
            && search.error.is_none()
            && !search.results.is_empty()
        {
            let offset = search.query.chars().count().saturating_sub(1);
            let mut out = io::stdout();

            for highlight in &search.results {
                let start = highlight.column_index;
                let end = highlight.column_index + offset;
                self.draw_highlight(
                    &Vec2::new(start, highlight.line_index),
                    &Vec2::new(end, highlight.line_index),
                    &SEARCH_HIGHLIGHT_FG_COLOR,
                    &SEARCH_HIGHLIGHT_BG_COLOR,
                    &mut out,
                )?;
            }
            // Only transition to Highlighted if not in Typing mode (real-time search)
            if search.state != SearchState::Typing {
                search.state = SearchState::Highlighted;
            }
        }
        self.search = Some(search);
        Ok(())
    }
}
