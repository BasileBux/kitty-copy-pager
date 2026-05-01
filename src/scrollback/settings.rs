use crossterm::style::Color;

pub(crate) const PROMPT_CURSOR_OFFSET: usize = 1;
pub(crate) const SCROLLOFF: usize = 4;
pub(crate) const SCROLL_JUMP: usize = 10; // TODO: make this dynamic based on the terminal height
pub(crate) const TAB_WIDTH: usize = 8;

pub(crate) const REAL_TIME_SEARCH: bool = true;
pub(crate) const SMARTCASE_SEARCH: bool = true;

pub(crate) const STATUS_LINE_BG_COLOR: Color = Color::DarkGrey;
pub(crate) const STATUS_LINE_FG_COLOR: Color = Color::White;

pub(crate) const SEARCH_ERROR_FG_COLOR: Color = Color::Red;

pub(crate) const SELECTION_BG_COLOR: Color = Color::Yellow;
pub(crate) const SELECTION_FG_COLOR: Color = Color::Black;

pub(crate) const SEARCH_HIGHLIGHT_BG_COLOR: Color = Color::Blue;
pub(crate) const SEARCH_HIGHLIGHT_FG_COLOR: Color = Color::Black;

pub struct Settings {
    pub prompt_cursor_offset: usize,
    pub scrolloff: usize,
    pub scroll_jump: usize,
    pub tab_width: usize,

    pub status_line_bg_color: Color,
    pub status_line_fg_color: Color,
    pub search_error_fg_color: Color,
    pub selection_bg_color: Color,
    pub selection_fg_color: Color,
    pub search_highlight_bg_color: Color,
    pub search_highlight_fg_color: Color,

    pub real_time_search: bool,
    pub smartcase_search: bool,
}

pub struct OptSettings {
    pub prompt_cursor_offset: Option<usize>,
    pub scrolloff: Option<usize>,
    pub scroll_jump: Option<usize>,
    pub tab_width: Option<usize>,

    pub status_line_bg_color: Option<Color>,
    pub status_line_fg_color: Option<Color>,
    pub search_error_fg_color: Option<Color>,
    pub selection_bg_color: Option<Color>,
    pub selection_fg_color: Option<Color>,
    pub search_highlight_bg_color: Option<Color>,
    pub search_highlight_fg_color: Option<Color>,

    pub real_time_search: Option<bool>,
    pub smartcase_search: Option<bool>,
}

impl Settings {
    pub fn new() -> Self {
        Self {
            prompt_cursor_offset: PROMPT_CURSOR_OFFSET,
            scrolloff: SCROLLOFF,
            scroll_jump: SCROLL_JUMP,
            tab_width: TAB_WIDTH,

            status_line_bg_color: STATUS_LINE_BG_COLOR,
            status_line_fg_color: STATUS_LINE_FG_COLOR,
            search_error_fg_color: SEARCH_ERROR_FG_COLOR,
            selection_bg_color: SELECTION_BG_COLOR,
            selection_fg_color: SELECTION_FG_COLOR,
            search_highlight_bg_color: SEARCH_HIGHLIGHT_BG_COLOR,
            search_highlight_fg_color: SEARCH_HIGHLIGHT_FG_COLOR,

            real_time_search: REAL_TIME_SEARCH,
            smartcase_search: SMARTCASE_SEARCH,
        }
    }

    pub fn from_opt(opt: OptSettings) -> Self {
        let default = Self::new();
        Self {
            prompt_cursor_offset: opt
                .prompt_cursor_offset
                .unwrap_or(default.prompt_cursor_offset),
            scrolloff: opt.scrolloff.unwrap_or(default.scrolloff),
            scroll_jump: opt.scroll_jump.unwrap_or(default.scroll_jump),
            tab_width: opt.tab_width.unwrap_or(default.tab_width),

            status_line_bg_color: opt
                .status_line_bg_color
                .unwrap_or(default.status_line_bg_color),
            status_line_fg_color: opt
                .status_line_fg_color
                .unwrap_or(default.status_line_fg_color),
            search_error_fg_color: opt
                .search_error_fg_color
                .unwrap_or(default.search_error_fg_color),
            selection_bg_color: opt.selection_bg_color.unwrap_or(default.selection_bg_color),
            selection_fg_color: opt.selection_fg_color.unwrap_or(default.selection_fg_color),
            search_highlight_bg_color: opt
                .search_highlight_bg_color
                .unwrap_or(default.search_highlight_bg_color),
            search_highlight_fg_color: opt
                .search_highlight_fg_color
                .unwrap_or(default.search_highlight_fg_color),

            real_time_search: opt.real_time_search.unwrap_or(default.real_time_search),
            smartcase_search: opt.smartcase_search.unwrap_or(default.smartcase_search),
        }
    }
}
