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
    pub current_result_index: usize,
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
                current_result_index: 0,
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

    pub(crate) fn get_next_match(&self) -> Option<&SearchResult> {
        if let Some(search) = &self.search {
            if search.error.is_some() || search.results.is_empty() {
                return None;
            }
            Some(
                &search.results
                    [search.current_result_index.saturating_add(1) % search.results.len()],
            )
        } else {
            None
        }
    }

    pub(crate) fn move_to_next_match(&mut self) -> io::Result<()> {
        match self.get_next_match() {
            Some(result) => {
                self.move_to(result.column_index, result.line_index)?;
                if let Some(search) = &mut self.search {
                    search.current_result_index =
                        (search.current_result_index + 1) % search.results.len();
                }
            }
            None => {
                self.search = Some(Search {
                    query: String::new(),
                    state: SearchState::Hidden,
                    results: Vec::new(),
                    error: Some("Error: No search query".to_string()),
                    current_result_index: 0,
                });
            }
        }
        Ok(())
    }

    pub(crate) fn get_prev_match(&self) -> Option<&SearchResult> {
        if let Some(search) = &self.search {
            if search.error.is_some() || search.results.is_empty() {
                return None;
            }
            let prev_index = search
                .current_result_index
                .checked_sub(1)
                .unwrap_or(search.results.len() - 1);
            Some(&search.results[prev_index])
        } else {
            None
        }
    }

    pub(crate) fn move_to_prev_match(&mut self) -> io::Result<()> {
        match self.get_prev_match() {
            Some(result) => {
                self.move_to(result.column_index, result.line_index)?;
                if let Some(search) = &mut self.search {
                    search.current_result_index = search
                        .current_result_index
                        .checked_sub(1)
                        .unwrap_or(search.results.len() - 1);
                }
            }
            None => {
                self.search = Some(Search {
                    query: String::new(),
                    state: SearchState::Hidden,
                    results: Vec::new(),
                    error: Some("Error: No search query".to_string()),
                    current_result_index: 0,
                });
            }
        }
        Ok(())
    }

    pub(crate) fn get_closest_next_match(&self) -> Option<(&SearchResult, usize)> {
        if let Some(search) = &self.search {
            if search.error.is_some() {
                return None;
            }
            for (i, result) in search.results.iter().enumerate() {
                if result.line_index > self.logical_y
                    || (result.line_index == self.logical_y && result.column_index > self.cursor_x)
                {
                    return Some((result, i));
                }
            }
            return Some((&search.results[0], 0));
        }
        None
    }

    pub(crate) fn move_to_closest_match(&mut self) -> io::Result<()> {
        match self.get_closest_next_match() {
            Some((result, idx)) => {
                self.move_to(result.column_index, result.line_index)?;
                if let Some(search) = &mut self.search {
                    search.current_result_index = idx;
                }
            }
            None => {
                self.search = Some(Search {
                    query: String::new(),
                    state: SearchState::Hidden,
                    results: Vec::new(),
                    error: Some("Error: No search results".to_string()),
                    current_result_index: 0,
                });
                self.draw_status_line()?;
            }
        }
        Ok(())
    }
}
