use super::Pager;

use crate::pager::search::SearchState;
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

impl Pager {
    pub fn draw_status_line(&mut self) -> io::Result<()> {
        let mut out = stdout();
        let status_line_y = self.term_height as u16;
        let status_text = format!("Ln {}, Col {}", self.logical_y, self.cursor_x);

        out.queue(MoveTo(0, status_line_y))?;
        out.queue(SetBackgroundColor(self.settings.status_line_bg_color))?;
        out.queue(SetForegroundColor(self.settings.status_line_fg_color))?;
        out.queue(Clear(ClearType::CurrentLine))?;

        let mut cursor_x = self.get_physical_cursor_x() as u16;
        let mut cursor_y = self.get_physical_cursor_y() as u16;

        let Some(search) = &mut self.search else {
            out.queue(MoveTo(
                self.term_width.saturating_sub(status_text.width()) as u16,
                status_line_y,
            ))?;
            out.queue(Print(&status_text))?;
            out.queue(MoveTo(cursor_x, cursor_y))?;
            return out.flush();
        };

        let match_counter = if search.error.is_none() && !search.results.is_empty() {
            format!("[{}/{}] ", search.current_result_index + 1, search.results.len())
        } else {
            String::new()
        };

        let extra_width = status_text.width() + match_counter.width();
        let query_width = search.query.width();
        let threshold = self.term_width.saturating_sub(extra_width * 2);
        let is_long = query_width > threshold;
        let num_lines = (query_width as f64 / self.term_width.saturating_sub(1) as f64).ceil()
            as usize;

        if is_long && num_lines > self.term_height as usize / 2 {
            search.error = Some("Search query too long to display".to_string());
        }

        search.long_search = is_long && search.error.is_none();

        if let Some(error) = &search.error {
            out.queue(SetForegroundColor(self.settings.search_error_fg_color))?;
            out.queue(Print(error))?;
            out.queue(SetForegroundColor(self.settings.status_line_fg_color))?;
            out.queue(MoveTo(cursor_x, cursor_y))?;
            return out.flush();
        }

        if search.state == SearchState::Hidden {
            out.queue(MoveTo(
                self.term_width.saturating_sub(status_text.width()) as u16,
                status_line_y,
            ))?;
            out.queue(Print(&status_text))?;
            out.queue(MoveTo(cursor_x, cursor_y))?;
            return out.flush();
        }

        let prompt = if search.state == SearchState::Typing {
            cursor_y = status_line_y;
            if is_long {
                cursor_x = (query_width % self.term_width) as u16 + 1;
                out.queue(MoveTo(0, status_line_y.saturating_sub(num_lines as u16)))?;
            } else {
                cursor_x = query_width as u16 + 1;
            }
            search.query.as_str()
        } else if is_long {
            let available = self.term_width.saturating_sub(
                STATUS_LINE_AVG_LEN * 2 + match_counter.width() + PROMPT_ELIPSIS.width() + 1,
            );
            &search.query[..get_utf_index(&search.query, available)]
        } else {
            search.query.as_str()
        };

        out.queue(MoveTo(0, status_line_y))?;
        out.queue(Print(format!("/{}", prompt)))?;
        if is_long && search.state != SearchState::Typing {
            out.queue(Print(PROMPT_ELIPSIS))?;
        }

        if !is_long {
            if !match_counter.is_empty() {
                let counter_x = self
                    .term_width
                    .saturating_sub(status_text.width() + match_counter.width());
                out.queue(MoveTo(counter_x as u16, status_line_y))?;
                out.queue(Print(&match_counter))?;
            }
            let status_x = self.term_width.saturating_sub(status_text.width());
            out.queue(MoveTo(status_x as u16, status_line_y))?;
            out.queue(Print(&status_text))?;
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

        out.queue(SetAttribute(Attribute::Reset))?;
        out.queue(SetForegroundColor(*fg_color))?;
        out.queue(SetBackgroundColor(*bg_color))?;

        // Single-line highlight
        if start.y == end.y {
            let line = &self.text_lines[start.y];
            let start_idx = get_utf_index(line, start.x);
            let end_idx = get_utf_index(line, end.x + 1);
            let screen_y = (start.y - self.viewport_start) as u16;
            out.queue(MoveTo(start.x as u16, screen_y))?;
            out.queue(Print(&line[start_idx..end_idx]))?;
            out.queue(SetForegroundColor(Color::Reset))?;
            out.queue(SetBackgroundColor(Color::Reset))?;
            return Ok(());
        }

        // Multi-line highlight
        let first_visible_y = start.y.max(self.viewport_start);
        let last_visible_y = end.y.min(self.viewport_end);

        // Draw first line (from start.x to end of line)
        if start.y >= self.viewport_start && start.y < self.text_lines.len() {
            let line = &self.text_lines[start.y];
            let start_idx = get_utf_index(line, start.x);
            let screen_y = (start.y - self.viewport_start) as u16;
            out.queue(MoveTo(start.x as u16, screen_y))?;
            out.queue(Print(&line[start_idx..]))?;
        }

        // Draw full middle lines
        for line_y in (first_visible_y + 1)..last_visible_y.min(self.text_lines.len()) {
            let line = &self.text_lines[line_y];
            let screen_y = (line_y - self.viewport_start) as u16;
            out.queue(MoveTo(0, screen_y))?;
            out.queue(Print(if line.is_empty() { " " } else { line }))?;
        }

        // Draw last line (from start to end.x)
        if end.y <= self.viewport_end && end.y < self.text_lines.len() {
            let line = &self.text_lines[end.y];
            let end_idx = get_utf_index(line, end.x + 1);
            let screen_y = (end.y - self.viewport_start) as u16;
            out.queue(MoveTo(0, screen_y))?;
            out.queue(Print(&line[..end_idx]))?;
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
                &self.settings.selection_fg_color,
                &self.settings.selection_bg_color,
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
            || (search.state == SearchState::Typing && self.settings.real_time_search)
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
                    &self.settings.search_highlight_fg_color,
                    &self.settings.search_highlight_bg_color,
                    &mut out,
                )?;
            }
            if search.state != SearchState::Typing {
                search.state = SearchState::Highlighted;
            }
        }
        self.search = Some(search);
        Ok(())
    }
}
