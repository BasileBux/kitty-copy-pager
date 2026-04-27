use super::ScrollbackBuffer;

use crate::scrollback::search::SearchState;
use crate::scrollback::{
    SEARCH_ERROR_FG_COLOR, SEARCH_HIGHLIGHT_BG_COLOR, SEARCH_HIGHLIGHT_FG_COLOR,
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

impl ScrollbackBuffer {
    pub fn draw_status_line(&self) -> io::Result<()> {
        let mut out = stdout();
        out.queue(MoveTo(0, self.term_height as u16))?;
        out.queue(SetBackgroundColor(STATUS_LINE_BG_COLOR))?;
        out.queue(SetForegroundColor(STATUS_LINE_FG_COLOR))?;
        out.queue(Clear(ClearType::CurrentLine))?;

        let mut cursor_x = self.get_physical_cursor_x() as u16;
        let mut cursor_y = self.get_physical_cursor_y() as u16;

        if let Some(search) = &self.search {
            out.queue(MoveTo(0, self.term_height as u16))?;
            if let Some(error) = &search.error {
                out.queue(SetForegroundColor(SEARCH_ERROR_FG_COLOR))?;
                out.queue(Print(error.as_str()))?;
                out.queue(SetForegroundColor(STATUS_LINE_FG_COLOR))?;
            } else {
                if search.state == SearchState::Typing {
                    cursor_y = self.term_height as u16;
                    cursor_x = search.query.width() as u16 + 1;
                }
                out.queue(Print(&format!("/{}", search.query)))?;
            }
        }

        let status_text = format!("Ln {}, Col {}", self.logical_y, self.cursor_x);
        out.queue(MoveTo(
            self.term_width.saturating_sub(status_text.width()) as u16,
            self.term_height as u16,
        ))?;
        out.queue(Print(status_text))?;

        out.queue(MoveTo(cursor_x, cursor_y))?;
        out.flush()
    }

    pub fn draw(&self) -> io::Result<()> {
        let mut out = stdout();
        out.queue(SetBackgroundColor(Color::Reset))?;
        out.queue(Clear(ClearType::All))?;
        for (i, line) in self.lines[self.viewport_start..self.viewport_end.saturating_add(1)]
            .iter()
            .enumerate()
        {
            out.queue(MoveTo(0, i as u16))?;
            out.queue(Print(line))?;
        }
        if let Some(sel) = &self.selection {
            self.draw_highlight(
                &sel.start,
                &sel.end,
                &Color::Black,
                &Color::Yellow,
                &mut out,
            )?;
        }
        self.draw_status_line()?;
        self.draw_search()?;

        out.queue(MoveTo(
            self.get_physical_cursor_x() as u16,
            self.get_physical_cursor_y() as u16,
        ))?;
        out.flush()
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
        if let Some(sel) = &self.selection {
            self.draw_highlight(
                &sel.start,
                &sel.end,
                &SELECTION_FG_COLOR,
                &SELECTION_BG_COLOR,
                &mut out,
            )?;
        }
        out.flush()?;
        Ok(())
    }

    fn get_physical_cursor_x(&self) -> usize {
        self.current_line()[..get_utf_index(self.current_line(), self.cursor_x)].width()
    }

    pub(crate) fn get_physical_cursor_y(&self) -> usize {
        self.logical_y.saturating_sub(self.viewport_start)
    }

    pub(crate) fn draw_search(&self) -> io::Result<()> {
        if let Some(search) = &self.search
            && search.state == SearchState::Highlighted
        {
            for highlight in search.results.iter() {
                let start = highlight.column_index;
                let end = highlight.column_index + search.query.chars().count().saturating_sub(1);
                self.draw_highlight(
                    &Vec2::new(start, highlight.line_index),
                    &Vec2::new(end, highlight.line_index),
                    &SEARCH_HIGHLIGHT_FG_COLOR,
                    &SEARCH_HIGHLIGHT_BG_COLOR,
                    &mut stdout(),
                )?;
            }
        }
        Ok(())
    }
}
