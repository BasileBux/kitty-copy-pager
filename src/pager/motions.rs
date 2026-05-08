use super::Pager;

use crate::utils::{VimCharExt, get_utf_index};
use crossterm::event::KeyCode;
use std::cmp::min;
use std::io::{self};

// TODO: Simplify the logic in these functions, it's really convoluted at the moment.

impl Pager {
    pub(crate) fn movement_underscore(&mut self) -> io::Result<()> {
        let mut jmp = 0;
        for (i, c) in self.current_line().chars().enumerate() {
            if !c.is_whitespace() {
                jmp = i;
                break;
            }
        }
        self.move_horizontally_to(jmp)
    }

    #[inline(always)]
    fn jump_condition(c: char, prev: char) -> bool {
        // Case 1: 'a,'
        // Case 2: ',a' (but not ', ')
        (c.is_vim_punctuation() && !prev.is_vim_punctuation())
            || (!c.is_vim_punctuation() && !c.is_whitespace() && prev.is_vim_punctuation())
    }

    fn find_w_jump(&self, whitespace_only: bool, already_wrapped: bool) -> Option<usize> {
        let line = self.current_line();
        let start = get_utf_index(&line, self.cursor_x);
        for (i, c) in line[start..].chars().enumerate() {
            let prev = line
                .chars()
                .nth(self.cursor_x.saturating_add(i.saturating_sub(1)))
                .unwrap_or('a');
            let idx = self.cursor_x.saturating_add(i);

            // Whitespace check or punctuation check depending on the mode
            if (!c.is_whitespace() && (prev.is_whitespace() || (idx == 0 && already_wrapped)))
                || (!whitespace_only && Self::jump_condition(c, prev))
            {
                return Some(i);
            }
        }
        None
    }

    pub(crate) fn movement_w(
        &mut self,
        whitespace_only: bool,
        already_wrapped: bool,
    ) -> io::Result<()> {
        match self.find_w_jump(whitespace_only, already_wrapped) {
            Some(jump) => {
                self.move_horizontally_by(jump as isize)?;
            }
            None => {
                if !already_wrapped {
                    self.wrap_to_next(whitespace_only, true, Pager::movement_w)?;
                }
            }
        }
        Ok(())
    }

    // BUG: When last char is a valid jump, it will always jump one too far
    fn find_b_jump(&self, whitespace_only: bool, already_wrapped: bool) -> Option<usize> {
        let line = self.current_line();
        let end = if !already_wrapped {
            get_utf_index(&line, self.cursor_x)
        } else {
            line.len()
        };
        for (i, c) in line[..end].chars().rev().enumerate() {
            let idx = self.cursor_x.saturating_sub(i);
            let prev = line.chars().nth(idx.saturating_sub(2)).unwrap_or('a');
            // Whitespace check or punctuation check depending on the mode
            if (!c.is_whitespace() && (prev.is_whitespace() || (idx == 1 && self.cursor_x != 0)))
                || (!whitespace_only && Self::jump_condition(c, prev))
            {
                return Some(i.saturating_add(1));
            }
        }
        None
    }

    pub(crate) fn movement_b(
        &mut self,
        whitespace_only: bool,
        already_wrapped: bool,
    ) -> io::Result<()> {
        match self.find_b_jump(whitespace_only, already_wrapped) {
            Some(jump) => {
                self.move_horizontally_by(-(jump as isize))?;
            }
            None => {
                if !already_wrapped {
                    self.wrap_to_previous(whitespace_only, true, Pager::movement_b)?;
                }
            }
        }
        Ok(())
    }

    pub(crate) fn movement_e(
        &mut self,
        whitespace: bool,
        mut already_wrapped: bool,
    ) -> io::Result<()> {
        let line = &self.current_line();
        let start = get_utf_index(
            &line,
            self.cursor_x.saturating_add(1 * !already_wrapped as usize),
        );
        let line_end = &line[start..];
        let mut len = line_end.chars().enumerate().count();
        if len <= 0 && !already_wrapped {
            already_wrapped = true;
            return self.wrap_to_next(whitespace, already_wrapped, Pager::movement_e);
        }
        len = len.saturating_sub(1);
        for (i, c) in line_end.chars().enumerate() {
            let peek = line_end.chars().nth(i + 1).unwrap_or('a');
            if (!c.is_whitespace()
                && !c.is_vim_punctuation()
                && (peek.is_whitespace() || (peek.is_vim_punctuation() && !whitespace)))
                || (c.is_vim_punctuation() && !peek.is_vim_punctuation() && !whitespace)
                || i == len
            {
                return self.move_horizontally_by(
                    min(i + (1 * !already_wrapped as usize), len + 1) as isize,
                );
            }
        }
        Ok(())
    }

    pub(crate) fn movement_dollar(&mut self) -> io::Result<()> {
        self.move_horizontally_to(self.current_line_len().saturating_sub(1))
    }

    #[allow(non_snake_case)]
    pub(crate) fn movement_G(&mut self) -> io::Result<()> {
        self.move_to(self.wish_cursor_x, self.lines.len().saturating_sub(1))
    }

    pub(crate) fn movement_gg(&mut self) -> io::Result<bool> {
        if let Some(last) = self.input_buffer.iter().last()
            && last == &KeyCode::Char('g')
        {
            self.move_to(self.wish_cursor_x, 0)?;
            return Ok(true);
        }
        return Ok(false);
    }
}
