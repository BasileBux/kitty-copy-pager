use super::ScrollbackBuffer;
use super::*;
use std::cmp::min;
use std::io::{self};

type WrappingMotionFn = fn(&mut ScrollbackBuffer, bool, bool) -> io::Result<()>;

impl ScrollbackBuffer {
    pub(crate) fn move_to(&mut self, x: usize, y: usize) -> io::Result<()> {
        self.logical_y = min(y, self.lines.len().saturating_sub(1));
        let line_len = self.current_line_len();
        self.cursor_x = min(x, line_len.saturating_sub(1));
        self.wish_cursor_x = self.cursor_x;
        self.movement_suffix(true)
    }

    pub(crate) fn wrap_to_next(
        &mut self,
        whitespace: bool,
        already_wrapped: bool,
        motion: WrappingMotionFn,
    ) -> io::Result<()> {
        let mut wrapped = false;
        let y = self.logical_y.saturating_add(1);
        if y <= self.lines.len().saturating_sub(1) {
            self.logical_y = y;
            self.cursor_x = 0;
            wrapped = true;
        }
        self.movement_suffix(wrapped)?;
        motion(self, whitespace, already_wrapped)
    }

    pub(crate) fn wrap_to_previous(
        &mut self,
        whitespace: bool,
        already_wrapped: bool,
        motion: WrappingMotionFn,
    ) -> io::Result<()> {
        let mut wrapped = false;
        let y = self.logical_y.saturating_sub(1);
        if y > 0 {
            self.logical_y = y;
            self.cursor_x = self.current_line_len().saturating_sub(1);
            wrapped = true;
        }
        self.movement_suffix(wrapped)?;
        motion(self, whitespace, already_wrapped)
    }

    pub(crate) fn move_vertically_by(&mut self, amount: isize) -> io::Result<()> {
        self.logical_y = min(
            self.logical_y.saturating_add_signed(amount),
            self.lines.len().saturating_sub(1),
        );
        let line_len = self.current_line_len();
        self.cursor_x = min(self.wish_cursor_x, line_len.saturating_sub(1));
        self.movement_suffix(true)
    }

    #[allow(dead_code)]
    pub(crate) fn move_vertically_to(&mut self, y: usize) -> io::Result<()> {
        self.move_to(self.cursor_x, y)?;
        let line_len = self.current_line_len();
        self.cursor_x = min(self.wish_cursor_x, line_len.saturating_sub(1));
        self.movement_suffix(true)
    }

    pub(crate) fn move_horizontally_by(&mut self, amount: isize) -> io::Result<()> {
        let line_len = self.current_line_len();
        self.cursor_x = min(
            self.cursor_x.saturating_add_signed(amount),
            line_len.saturating_sub(1),
        );
        self.wish_cursor_x = self.cursor_x;
        self.movement_suffix(false)
    }

    pub(crate) fn move_horizontally_to(&mut self, x: usize) -> io::Result<()> {
        self.move_to(x, self.logical_y)?;
        self.wish_cursor_x = self.cursor_x;
        self.movement_suffix(false)
    }

    pub(crate) fn move_viewport(&mut self) -> bool {
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

    pub(crate) fn movement_suffix(&mut self, rerender: bool) -> io::Result<()> {
        let viewport_moved = self.move_viewport();
        if (rerender && viewport_moved) || self.selection.is_some() {
            self.expand_selection();
            self.draw()
        } else {
            self.draw_status_line()?;
            self.draw_search(true)?;
            self.draw_cursor()
        }
    }
}
