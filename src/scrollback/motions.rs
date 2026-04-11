use super::ScrollbackBuffer;

use crate::utils::{VimCharExt, first_non_whitespace_idx_on, get_utf_index};
use crossterm::event::KeyCode;
use std::cmp::min;
use std::io::{self};

impl ScrollbackBuffer {
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

    pub(crate) fn movement_w(
        &mut self,
        whitespace: bool,
        mut already_wrapped: bool,
    ) -> io::Result<()> {
        let line = self.current_line();
        let cursor_char = line.chars().nth(self.cursor_x).unwrap_or('a');
        let punctuated = already_wrapped || cursor_char.is_vim_punctuation();
        let mut spaced = already_wrapped || cursor_char.is_whitespace();
        let mut amount = 0;
        let start = get_utf_index(
            &line,
            self.cursor_x.saturating_add(1 * !already_wrapped as usize),
        );
        for (i, c) in line[start..].chars().enumerate() {
            let prev_is_punctuation = line
                .chars()
                .nth(self.cursor_x.saturating_add(i))
                .unwrap_or('a')
                .is_vim_punctuation();
            if (!whitespace && (c.is_vim_punctuation() && !prev_is_punctuation)
                || (spaced && !c.is_whitespace()))
                || (punctuated && !c.is_whitespace() && !c.is_vim_punctuation())
            {
                amount = i.saturating_add(1 * !already_wrapped as usize);
                break;
            }
            if !spaced && c.is_whitespace() {
                spaced = true;
            }
        }
        if amount != 0 {
            self.move_horizontally_by(amount as isize)
        } else if !already_wrapped {
            already_wrapped = true;
            self.wrap_to_next(whitespace, already_wrapped, ScrollbackBuffer::movement_w)
        } else {
            Ok(())
        }
    }

    pub(crate) fn movement_b(
        &mut self,
        whitespace: bool,
        mut already_wrapped: bool,
    ) -> io::Result<()> {
        let line = &self.current_line();
        let mut amount = 0;
        let end = if !already_wrapped {
            get_utf_index(&line, self.cursor_x)
        } else {
            line.len()
        };
        if (self.cursor_x == 0 || self.cursor_x <= first_non_whitespace_idx_on(&line).unwrap_or(1))
            && !already_wrapped
        {
            already_wrapped = true;
            return self.wrap_to_previous(
                whitespace,
                already_wrapped,
                ScrollbackBuffer::movement_b,
            );
        }
        let mut found = false;
        for (i, c) in line[..end].chars().rev().enumerate() {
            let index = self.cursor_x.saturating_sub(i);
            let peek = line.chars().nth(index.saturating_sub(2)).unwrap_or('a');
            if !c.is_whitespace()
                && !c.is_vim_punctuation()
                && (peek.is_whitespace() || (peek.is_vim_punctuation() && !whitespace))
            {
                found = true;
                amount = min(i + 1, end);
                break;
            }

            if c.is_vim_punctuation() && !peek.is_vim_punctuation() && !whitespace {
                found = true;
                amount = if !already_wrapped { min(i + 1, end) } else { i };
                break;
            }
        }
        if !found {
            amount = self
                .cursor_x
                .saturating_sub(first_non_whitespace_idx_on(&line).unwrap_or(0));
        }
        self.move_horizontally_by(-(amount as isize))
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
            return self.wrap_to_next(whitespace, already_wrapped, ScrollbackBuffer::movement_e);
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
