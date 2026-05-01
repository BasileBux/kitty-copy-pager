use super::{REAL_TIME_SEARCH, SMARTCASE_SEARCH, ScrollbackBuffer};
use crossterm::event::{KeyCode, KeyEvent};
use regex::Regex;
use std::io::{self};
use unicode_width::UnicodeWidthStr;

pub(crate) struct SearchResult {
    pub line_index: usize,
    pub column_index: usize,
}

#[derive(PartialEq, Clone)]
pub(crate) enum SearchState {
    Typing,
    PendingRedraw,
    Highlighted,
    Hidden,
}

pub(crate) struct Search {
    pub query: String,
    pub state: SearchState,
    pub results: Vec<SearchResult>,
    pub error: Option<String>,
    pub current_result_index: usize,
    pub long_search: bool,
}

impl ScrollbackBuffer {
    /// Builds regex pattern with smartcase support.
    /// If SMARTCASE_SEARCH is enabled and query has no uppercase chars,
    /// prepends (?i) for case-insensitive matching.
    fn build_search_pattern(query: &str) -> String {
        if SMARTCASE_SEARCH && !query.chars().any(|c| c.is_uppercase()) {
            format!("(?i){}", query)
        } else {
            query.to_string()
        }
    }

    pub(crate) fn search_mode(&mut self, event: KeyEvent) -> io::Result<bool> {
        let get_lines = |query_width: usize, term_width: usize, long_search: bool| -> usize {
            if long_search {
                (query_width as f64 / term_width.saturating_sub(1) as f64).ceil() as usize
            } else {
                1
            }
        };

        let before_lines = if let Some(search) = &self.search
            && !REAL_TIME_SEARCH
        {
            get_lines(
                search.query.width(),
                self.term_width as usize,
                search.long_search,
            )
        } else {
            1
        };

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
                    if REAL_TIME_SEARCH {
                        self.search_realtime();
                    }
                }
                KeyCode::Backspace => {
                    search.query.pop();
                    if REAL_TIME_SEARCH {
                        self.search_realtime();
                    }
                }
                KeyCode::Esc => {
                    search.state = SearchState::Hidden;
                    self.draw()?;
                    return Ok(false);
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
                long_search: false,
            });
            self.draw_status_line()?;
            return Ok(false);
        }

        let after_lines = if let Some(search) = &self.search
            && !REAL_TIME_SEARCH
        {
            get_lines(
                search.query.width(),
                self.term_width as usize,
                search.long_search,
            )
        } else {
            1
        };

        let needs_full_redraw = after_lines < before_lines;

        if REAL_TIME_SEARCH || needs_full_redraw {
            self.draw()?;
        } else {
            self.draw_status_line()?;
        }
        Ok(false)
    }

    /// Performs real-time search while typing.
    /// Silently ignores regex errors (for incomplete patterns) and clears results instead.
    /// Note: Keeps state as Typing to stay in search mode.
    pub(crate) fn search_realtime(&mut self) {
        match &mut self.search {
            Some(search) => {
                search.error = None;
                if search.query.is_empty() {
                    search.results.clear();
                    search.state = SearchState::Typing;
                    return;
                }
                let pattern = Self::build_search_pattern(&search.query);
                match Regex::new(&pattern) {
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
                        search.state = SearchState::Typing;
                    }
                    Err(_) => {
                        search.results.clear();
                        search.state = SearchState::Typing;
                    }
                }
            }
            _ => return,
        }
    }

    pub(crate) fn search(&mut self) {
        match &mut self.search {
            Some(search) => {
                search.state = SearchState::Hidden;
                if search.query.is_empty() {
                    search.results.clear();
                    search.state = SearchState::Hidden;
                    search.error = Some("Error: empty search query".to_string());
                    return;
                }

                let pattern = Self::build_search_pattern(&search.query);
                match Regex::new(&pattern) {
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
                            search.results.clear();
                            search.state = SearchState::Hidden;
                            search.error =
                                Some("Error: could not find any occurrences".to_string());
                            return;
                        }
                    }
                    Err(_) => {
                        search.results.clear();
                        search.state = SearchState::Hidden;
                        search.error = Some("Error: Invalid regex pattern".to_string());
                        return;
                    }
                }
                search.state = SearchState::PendingRedraw;
            }
            _ => return,
        }
    }

    pub(crate) fn move_to_next_match(&mut self) -> io::Result<()> {
        let next = if let Some(search) = &self.search {
            if search.error.is_some() || search.results.is_empty() {
                None
            } else {
                let next_index = (search.current_result_index + 1) % search.results.len();
                Some((
                    search.results[next_index].column_index,
                    search.results[next_index].line_index,
                    next_index,
                ))
            }
        } else {
            None
        };

        match next {
            Some((col, line, idx)) => {
                if let Some(search) = &mut self.search {
                    search.current_result_index = idx;
                    search.state = SearchState::Highlighted;
                }
                self.move_to(col, line)?;
            }
            None => {
                self.search = Some(Search {
                    query: String::new(),
                    state: SearchState::Hidden,
                    results: Vec::new(),
                    error: Some("Error: No search query".to_string()),
                    current_result_index: 0,
                    long_search: false,
                });
            }
        }
        Ok(())
    }

    pub(crate) fn move_to_prev_match(&mut self) -> io::Result<()> {
        let prev = if let Some(search) = &self.search {
            if search.error.is_some() || search.results.is_empty() {
                None
            } else {
                let prev_index = search
                    .current_result_index
                    .checked_sub(1)
                    .unwrap_or(search.results.len() - 1);
                Some((
                    search.results[prev_index].column_index,
                    search.results[prev_index].line_index,
                    prev_index,
                ))
            }
        } else {
            None
        };

        match prev {
            Some((col, line, idx)) => {
                if let Some(search) = &mut self.search {
                    search.current_result_index = idx;
                    search.state = SearchState::Highlighted;
                }
                self.move_to(col, line)?;
            }
            None => {
                self.search = Some(Search {
                    query: String::new(),
                    state: SearchState::Hidden,
                    results: Vec::new(),
                    error: Some("Error: No search query".to_string()),
                    current_result_index: 0,
                    long_search: false,
                });
            }
        }
        Ok(())
    }

    pub(crate) fn get_closest_next_match(&self) -> Option<(&SearchResult, usize)> {
        if let Some(search) = &self.search {
            if search.error.is_some() || search.results.is_empty() {
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

    pub(crate) fn get_closest_prev_match(&self) -> Option<(&SearchResult, usize)> {
        if let Some(search) = &self.search {
            if search.error.is_some() || search.results.is_empty() {
                return None;
            }
            for (i, result) in search.results.iter().enumerate().rev() {
                if result.line_index < self.logical_y
                    || (result.line_index == self.logical_y && result.column_index < self.cursor_x)
                {
                    return Some((result, i));
                }
            }
            return Some((
                &search.results[search.results.len() - 1],
                search.results.len() - 1,
            ));
        }
        None
    }

    pub(crate) fn move_to_closest_match(&mut self, next: bool) -> io::Result<()> {
        let closest_match = if next {
            self.get_closest_next_match()
        } else {
            self.get_closest_prev_match()
        };
        match closest_match {
            Some((result, idx)) => {
                self.move_to(result.column_index, result.line_index)?;
                if let Some(search) = &mut self.search {
                    search.current_result_index = idx;
                    search.state = SearchState::Highlighted;
                }
            }
            None => {
                self.search = Some(Search {
                    query: String::new(),
                    state: SearchState::Hidden,
                    results: Vec::new(),
                    error: Some("Error: No search results".to_string()),
                    current_result_index: 0,
                    long_search: false,
                });
                self.draw_status_line()?;
            }
        }
        Ok(())
    }

    pub(crate) fn move_to_closest_next_match(&mut self) -> io::Result<()> {
        self.move_to_closest_match(true)
    }

    pub(crate) fn move_to_closest_prev_match(&mut self) -> io::Result<()> {
        self.move_to_closest_match(false)
    }
}
