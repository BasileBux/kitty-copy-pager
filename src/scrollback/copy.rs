use super::ScrollbackBuffer;

use crate::selection::*;
use crate::utils::get_utf_index;
use crossterm::{clipboard::CopyToClipboard, execute};
use std::io::{self, Write, stdout};
use std::{cmp::min};

impl ScrollbackBuffer {
    pub(crate) fn expand_selection(&mut self) {
        let y = self.logical_y;
        if let Some(sel) = &mut self.selection {
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
    }

    pub(crate) fn copy_selection(&self) -> io::Result<()> {
        if let Some(sel) = &self.selection {
            let mut copy_string = String::new();
            let end_y = sel.end.y.min(self.text_lines.len().saturating_sub(1));
            let last_i = end_y - sel.start.y;

            for (i, line) in self.text_lines[sel.start.y..=end_y].iter().enumerate() {
                if i == 0 && i == last_i {
                    let start = get_utf_index(line, sel.start.x);
                    let end = min(get_utf_index(line, sel.end.x), line.len().saturating_sub(1));
                    copy_string.push_str(&line[start..end + 1]);
                } else if i == 0 {
                    let start = get_utf_index(line, sel.start.x);
                    copy_string.push_str(&line[start..]);
                    copy_string.push_str("\n");
                } else if i == last_i {
                    let end = min(get_utf_index(line, sel.end.x), line.len().saturating_sub(2));
                    copy_string.push_str(&line[..end + 1]);
                    copy_string.push_str("\n");
                } else {
                    copy_string.push_str(line);
                    copy_string.push_str("\n");
                }
            }
            let mut out = stdout();
            execute!(out, CopyToClipboard::to_clipboard_from(&copy_string))?;
            out.flush()?;
        }
        Ok(())
    }
}
