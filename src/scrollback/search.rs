use super::ScrollbackBuffer;
use crate::scrollback::{STATUS_LINE_BG_COLOR, STATUS_LINE_FG_COLOR};
use crossterm::{
    QueueableCommand,
    event::{KeyCode, KeyEvent},
};
use regex::Regex;
use std::io::{self, Write};

pub(crate) struct SearchResult {
    pub line_index: usize,
    pub column_index: usize,
}

pub(crate) struct Search {
    pub query_size: usize, // Size is the same for all results, so we can store it once
    pub results: Vec<SearchResult>,
}

impl ScrollbackBuffer {
    pub(crate) fn search_mode(&mut self, event: KeyEvent) -> io::Result<bool> {
        let mut out = io::stdout();
        if self.search_query.is_none() {
            self.search_query = Some(String::new());
        } else {
            match event.code {
                KeyCode::Char(c) => {
                    self.search_query.as_mut().unwrap().push(c);
                }
                KeyCode::Backspace => {
                    if let Some(ref mut s) = self.search_query {
                        s.pop();
                    }
                }
                KeyCode::Esc => {
                    self.search_query = None;
                }
                KeyCode::Enter => {
                    return Ok(true);
                }
                _ => {}
            }
        }
        self.draw_status_line()?;
        out.queue(crossterm::cursor::MoveTo(0, self.term_height as u16))?;
        out.queue(crossterm::style::SetBackgroundColor(STATUS_LINE_BG_COLOR))?;
        out.queue(crossterm::style::SetBackgroundColor(STATUS_LINE_BG_COLOR))?;
        out.queue(crossterm::style::SetForegroundColor(STATUS_LINE_FG_COLOR))?;
        out.queue(crossterm::terminal::Clear(
            crossterm::terminal::ClearType::CurrentLine,
        ))?;
        let search_text = self.search_query.as_deref().unwrap_or("");
        out.queue(crossterm::style::Print(format!("/{}", search_text)))?;

        out.flush()?;
        Ok(false)
    }

    pub(crate) fn search(&mut self) -> Result<(), ()> {
        let query = self.search_query.as_ref().ok_or(())?;
        if query.is_empty() {
            return Err(());
        }

        let regex = Regex::new(query).map_err(|_| ())?;
        let mut results = Vec::new();

        for (line_index, line) in self.text_lines.iter().enumerate() {
            for mat in regex.find_iter(line) {
                // mat.start() is byte offset, convert to char count for column index
                let column_index = line[..mat.start()].chars().count();
                results.push(SearchResult {
                    line_index,
                    column_index,
                });
            }
        }

        if results.is_empty() {
            return Err(());
        }

        self.search = Some(Search {
            query_size: query.chars().count(),
            results,
        });
        self.search_query = None;

        Ok(())
    }
}
