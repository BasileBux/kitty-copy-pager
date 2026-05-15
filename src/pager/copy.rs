use super::Pager;

use crate::selection::*;
use crate::utils::get_utf_index;
use crossterm::{clipboard::CopyToClipboard, execute};
use std::cmp::min;
use std::io::{self, Write, stdout};

impl Pager {
    pub(crate) fn expand_selection(&mut self) {
        let y = self.logical_y;
        let Some(sel) = &mut self.selection else {
            return;
        };
        match sel.sel_end {
            SelectedEnd::Start => {
                if y > sel.end.y || (y == sel.end.y && self.cursor_x > sel.end.x) {
                    sel.swap_ends_to(self.cursor_x, y);
                    sel.sel_end = SelectedEnd::End;
                } else {
                    sel.start = Vec2::new(self.cursor_x, y);
                }
            }
            SelectedEnd::End => {
                if y < sel.start.y || (y == sel.start.y && self.cursor_x < sel.start.x) {
                    sel.swap_ends_to(self.cursor_x, y);
                    sel.sel_end = SelectedEnd::Start;
                } else {
                    sel.end = Vec2::new(self.cursor_x, y);
                }
            }
        }
    }

    pub(crate) fn copy_selection(&self) -> io::Result<()> {
        let Some(sel) = &self.selection else {
            return Ok(());
        };
        let mut copy_string = String::new();
        if sel.start == sel.end {
            let line = self.lines[sel.start.y].display();
            let char_index = get_utf_index(line, sel.start.x);
            let next_char_index = get_utf_index(line, sel.start.x + 1);
            copy_string.push_str(&line[char_index..next_char_index]);
            let mut out = stdout();
            execute!(out, CopyToClipboard::to_clipboard_from(&copy_string))?;
            out.flush()?;
            return Ok(());
        }
        let end_y = sel.end.y.min(self.lines.len().saturating_sub(1));
        let last_i = end_y - sel.start.y;

        for (i, line) in self.lines[sel.start.y..=end_y].iter().enumerate() {
            let raw_line = line.display();
            if i == 0 && i == last_i {
                let start = get_utf_index(raw_line, sel.start.x);
                let end = min(get_utf_index(raw_line, sel.end.x), raw_line.len().saturating_sub(1));
                copy_string.push_str(&raw_line[start..end + 1]);
            } else if i == 0 {
                let start = get_utf_index(raw_line, sel.start.x);
                copy_string.push_str(&raw_line[start..]);
                copy_string.push('\n');
            } else if i == last_i {
                let end = min(get_utf_index(raw_line, sel.end.x), raw_line.len().saturating_sub(1));
                copy_string.push_str(&raw_line[..end + 1]);
                copy_string.push('\n');
            } else {
                copy_string.push_str(raw_line);
                copy_string.push('\n');
            }
        }
        let mut out = stdout();
        execute!(out, CopyToClipboard::to_clipboard_from(&copy_string))?;
        out.flush()?;
        Ok(())
    }
}
