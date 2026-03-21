use super::ScrollbackBuffer;

use crate::selection::*;
use crate::utils::get_utf_index;
use crossterm::{
    QueueableCommand,
    cursor::MoveTo,
    style::{Color, Print, SetBackgroundColor, SetForegroundColor},
    terminal::{Clear, ClearType},
};
use std::io::{self, Write, stdout};
use std::{cmp::min, io::Stdout};

impl ScrollbackBuffer {
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

    pub(crate) fn draw_selection(&self, sel: &Selection, out: &mut Stdout) -> io::Result<()> {
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

    pub(crate) fn draw_cursor(&self) -> io::Result<()> {
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

    pub(crate) fn get_physical_cursor_y(&self) -> usize {
        self.logical_y.saturating_sub(self.viewport_start)
    }
}
