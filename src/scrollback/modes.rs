use super::ScrollbackBuffer;

use crossterm::event::{KeyCode, KeyEvent};
use std::io::{self};

use crate::scrollback::SCROLL_JUMP;
use crate::scrollback::search::SearchState;
use crate::selection::Selection;

impl ScrollbackBuffer {
    pub(crate) fn normal_mode(&mut self, event: KeyEvent) -> io::Result<bool> {
        match event.code {
            KeyCode::Char('q') => return Ok(true),

            // Simple movement
            KeyCode::Char('j') => self.move_vertically_by(1)?,
            KeyCode::Char('k') => self.move_vertically_by(-1)?,
            KeyCode::Char('d') => self.move_vertically_by(SCROLL_JUMP as isize)?, // Replaces ctrl+d
            KeyCode::Char('u') => self.move_vertically_by(-(SCROLL_JUMP as isize))?, // Replaces ctrl+u
            KeyCode::Char('h') => self.move_horizontally_by(-1)?,
            KeyCode::Char('l') => self.move_horizontally_by(1)?,

            // Movement to the end
            KeyCode::Char('0') => self.move_horizontally_to(0)?,
            KeyCode::Char('_') | KeyCode::Char('^') => self.movement_underscore()?,
            KeyCode::Char('$') => self.movement_dollar()?,

            // Movement to next/prev word
            KeyCode::Char('w') => self.movement_w(false, false)?,
            KeyCode::Char('W') => self.movement_w(true, false)?,
            KeyCode::Char('b') => self.movement_b(false, false)?,
            KeyCode::Char('B') => self.movement_b(true, false)?,
            KeyCode::Char('e') => self.movement_e(false, false)?,
            KeyCode::Char('E') => self.movement_e(true, false)?,

            // Top/Bottom movement
            KeyCode::Char('G') => self.movement_G()?,
            KeyCode::Char('g') => {
                if self.movement_gg()? {
                    self.movement_suffix(true)?;
                }
                self.input_buffer.push_back(event.code);
            }

            // Selection motions
            KeyCode::Char('v') => {
                self.selection = Some(Selection::with_coords(
                    self.cursor_x,
                    self.logical_y,
                    self.cursor_x,
                    self.logical_y,
                ));
            }
            KeyCode::Char('y') | KeyCode::Enter => {
                self.copy_selection()?;
                return Ok(true);
            }
            KeyCode::Esc => {
                if self.selection.is_some() {
                    self.selection = None;
                } else if let Some(search) = &mut self.search {
                    search.state = SearchState::Hidden;
                }
                self.draw()?;
            }

            // Search motions
            KeyCode::Char('/') => {
                self.search_mode(event)?;
            }
            KeyCode::Char('n') => {
                self.move_to_next_match()?;
            }
            KeyCode::Char('N') => {
                self.move_to_prev_match()?;
            }

            _ => {
                self.input_buffer.push_back(event.code);
            }
        }
        Ok(false)
    }
}
