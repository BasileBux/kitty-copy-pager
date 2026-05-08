use super::Pager;

use crate::utils::{VimCharExt, get_utf_index};
use crossterm::event::KeyCode;
use std::io::{self};

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
    fn jump_condition(c: char, peek: char) -> bool {
        // Case 1: 'a,'
        // Case 2: ',a' (but not ', ')
        (c.is_vim_punctuation() && !peek.is_vim_punctuation())
            || (!c.is_vim_punctuation() && !c.is_whitespace() && peek.is_vim_punctuation())
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

    fn find_b_jump(&self, whitespace_only: bool, already_wrapped: bool) -> Option<usize> {
        let line = self.current_line();
        // When wrapped, the cursor is at line_len-1 so we include it in the search.
        // When not wrapped, we search strictly before the cursor.
        let search_end = if already_wrapped {
            self.cursor_x.saturating_add(1)
        } else {
            self.cursor_x
        };
        let end = get_utf_index(&line, search_end);
        for (i, c) in line[..end].chars().rev().enumerate() {
            let target_idx = search_end.saturating_sub(1).saturating_sub(i);
            let prev = if target_idx == 0 {
                'a'
            } else {
                line.chars().nth(target_idx - 1).unwrap_or('a')
            };
            if (!c.is_whitespace()
                && (prev.is_whitespace() || (target_idx == 0 && self.cursor_x != 0)))
                || (!whitespace_only && Self::jump_condition(c, prev))
            {
                return Some(self.cursor_x.saturating_sub(target_idx));
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

    fn find_e_jump(&self, whitespace_only: bool, already_wrapped: bool) -> Option<usize> {
        let line = self.current_line();
        let search_start = if already_wrapped {
            0
        } else {
            self.cursor_x.saturating_add(1)
        };
        let start = get_utf_index(&line, search_start);
        let line_end = &line[start..];
        let last_idx = line_end.chars().count().saturating_sub(1);

        for (i, c) in line_end.chars().enumerate() {
            let peek = line_end.chars().nth(i + 1).unwrap_or('a');
            if i == last_idx
                || (!c.is_whitespace() && peek.is_whitespace())
                || (!whitespace_only && Self::jump_condition(peek, c))
            {
                let target_idx = search_start.saturating_add(i);
                return Some(target_idx.saturating_sub(self.cursor_x));
            }
        }
        None
    }

    pub(crate) fn movement_e(
        &mut self,
        whitespace_only: bool,
        already_wrapped: bool,
    ) -> io::Result<()> {
        match self.find_e_jump(whitespace_only, already_wrapped) {
            Some(jump) => {
                self.move_horizontally_by(jump as isize)?;
            }
            None => {
                if !already_wrapped {
                    self.wrap_to_next(whitespace_only, true, Pager::movement_e)?;
                }
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
