use super::ScrollbackBuffer;
use crossterm::event::{KeyCode, KeyEvent};
use regex::Regex;
use std::io::{self};

pub(crate) struct SearchResult {
    pub line_index: usize,
    pub column_index: usize,
}

#[derive(PartialEq)]
pub(crate) enum SearchState {
    Typing,
    Highlighted,
    Hidden,
}

pub(crate) struct Search {
    pub query: String,
    pub state: SearchState,
    pub results: Vec<SearchResult>,
    pub error: Option<String>,
}

impl ScrollbackBuffer {
    pub(crate) fn search_mode(&mut self, event: KeyEvent) -> io::Result<bool> {
        if let Some(search) = &mut self.search {
            if search.state != SearchState::Typing {
                search.state = SearchState::Typing;
                search.error = None;
                search.results.clear();
                search.query.clear();
                self.draw_status_line()?;
                return Ok(false);
            }

            match event.code {
                KeyCode::Char(c) => {
                    search.query.push(c);
                }
                KeyCode::Backspace => {
                    search.query.pop();
                }
                KeyCode::Esc => {
                    search.state = SearchState::Hidden;
                }
                KeyCode::Enter => {
                    return Ok(true);
                }
                _ => {}
            }
        } else {
            self.search = Some(Search {
                query: String::new(),
                state: SearchState::Typing,
                results: Vec::new(),
                error: None,
            });
        }

        self.draw_status_line()?;
        Ok(false)
    }

    pub(crate) fn search(&mut self) {
        match &mut self.search {
            Some(search) => {
                search.state = SearchState::Hidden;
                if search.query.is_empty() {
                    search.error = Some("Error: empty search query".to_string());
                    return;
                }

                match Regex::new(search.query.trim()) {
                    Ok(regex) => {
                        search.results.clear();

                        for (line_index, line) in self.text_lines.iter().enumerate() {
                            for mat in regex.find_iter(line) {
                                let column_index = line[..mat.start()].chars().count();
                                search.results.push(SearchResult {
                                    line_index,
                                    column_index,
                                });
                            }
                        }
                        if search.results.is_empty() {
                            search.error =
                                Some("Error: could not find any occurrences".to_string());
                            return;
                        }
                    }
                    Err(_) => {
                        search.error = Some("Error: Invalid regex pattern".to_string());
                        return;
                    }
                }
                search.state = SearchState::Highlighted;
            }
            _ => return,
        }
    }
}
