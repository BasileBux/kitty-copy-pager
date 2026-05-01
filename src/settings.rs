use clap::Parser;
use crossterm::style::Color;
use std::fmt;

#[derive(Debug, Clone, Copy)]
pub struct ColorArg(Color);

pub(crate) const PROMPT_CURSOR_OFFSET: usize = 1;
pub(crate) const SCROLLOFF: usize = 4;
pub(crate) const SCROLL_JUMP: usize = 10;
pub(crate) const TAB_WIDTH: usize = 8;

pub(crate) const STATUS_LINE_BG_COLOR: Color = Color::DarkGrey;
pub(crate) const STATUS_LINE_FG_COLOR: Color = Color::White;

pub(crate) const SEARCH_ERROR_FG_COLOR: Color = Color::Red;

pub(crate) const SELECTION_BG_COLOR: Color = Color::Yellow;
pub(crate) const SELECTION_FG_COLOR: Color = Color::Black;

pub(crate) const SEARCH_HIGHLIGHT_BG_COLOR: Color = Color::Blue;
pub(crate) const SEARCH_HIGHLIGHT_FG_COLOR: Color = Color::Black;

// realtime_search: true
// smartcase_search: true

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

impl Settings {
    pub fn from_args(args: Args) -> Self {
        Self {
            prompt_cursor_offset: args.prompt_cursor_offset,
            scrolloff: args.scrolloff,
            scroll_jump: SCROLL_JUMP,
            tab_width: args.tab_width,
            status_line_bg_color: args.status_line_bg_color.into(),
            status_line_fg_color: args.status_line_fg_color.into(),
            search_error_fg_color: args.search_error_fg_color.into(),
            selection_bg_color: args.selection_bg_color.into(),
            selection_fg_color: args.selection_fg_color.into(),
            search_highlight_bg_color: args.search_highlight_bg_color.into(),
            search_highlight_fg_color: args.search_highlight_fg_color.into(),
            real_time_search: !args.disable_real_time_search,
            smartcase_search: !args.disable_smartcase_search,
        }
    }
}

#[derive(Parser, Debug)]
pub struct Args {
    #[arg(long,  default_value_t = PROMPT_CURSOR_OFFSET)]
    pub prompt_cursor_offset: usize,

    #[arg(long, default_value_t = SCROLLOFF)]
    pub scrolloff: usize,

    #[arg(long, default_value_t = TAB_WIDTH)]
    pub tab_width: usize,

    #[arg(long, value_parser = parse_color, default_value_t = ColorArg(STATUS_LINE_BG_COLOR))]
    pub status_line_bg_color: ColorArg,

    #[arg(long, value_parser = parse_color, default_value_t = ColorArg(STATUS_LINE_FG_COLOR))]
    pub status_line_fg_color: ColorArg,

    #[arg(long, value_parser = parse_color, default_value_t = ColorArg(SEARCH_ERROR_FG_COLOR))]
    pub search_error_fg_color: ColorArg,

    #[arg(long, value_parser = parse_color, default_value_t = ColorArg(SELECTION_BG_COLOR))]
    pub selection_bg_color: ColorArg,

    #[arg(long, value_parser = parse_color, default_value_t = ColorArg(SELECTION_FG_COLOR))]
    pub selection_fg_color: ColorArg,

    #[arg(long, value_parser = parse_color, default_value_t = ColorArg(SEARCH_HIGHLIGHT_BG_COLOR))]
    pub search_highlight_bg_color: ColorArg,

    #[arg(long, value_parser = parse_color, default_value_t = ColorArg(SEARCH_HIGHLIGHT_FG_COLOR))]
    pub search_highlight_fg_color: ColorArg,

    #[arg(long, default_value_t = false)]
    pub disable_real_time_search: bool,

    #[arg(long, default_value_t = false)]
    pub disable_smartcase_search: bool,
}

fn parse_color(s: &str) -> Result<ColorArg, String> {
    match s.to_lowercase().as_str() {
        "black" => Ok(ColorArg(Color::Black)),
        "red" => Ok(ColorArg(Color::Red)),
        "green" => Ok(ColorArg(Color::Green)),
        "yellow" => Ok(ColorArg(Color::Yellow)),
        "blue" => Ok(ColorArg(Color::Blue)),
        "magenta" => Ok(ColorArg(Color::Magenta)),
        "cyan" => Ok(ColorArg(Color::Cyan)),
        "white" => Ok(ColorArg(Color::White)),
        "grey" | "gray" => Ok(ColorArg(Color::Grey)),
        "darkred" => Ok(ColorArg(Color::DarkRed)),
        "darkgreen" => Ok(ColorArg(Color::DarkGreen)),
        "darkyellow" => Ok(ColorArg(Color::DarkYellow)),
        "darkblue" => Ok(ColorArg(Color::DarkBlue)),
        "darkmagenta" => Ok(ColorArg(Color::DarkMagenta)),
        "darkcyan" => Ok(ColorArg(Color::DarkCyan)),
        "darkgrey" | "darkgray" => Ok(ColorArg(Color::DarkGrey)),
        // Ansi 256-color: "ansi:200"
        s if s.starts_with("ansi:") => {
            let n = s[5..]
                .parse::<u8>()
                .map_err(|_| format!("Invalid ANSI color index: '{}'", &s[5..]))?;
            Ok(ColorArg(Color::AnsiValue(n)))
        }
        // RGB hex: "#ff0080" or "rgb:255,0,128"
        s if s.starts_with('#') && s.len() == 7 => {
            let r = u8::from_str_radix(&s[1..3], 16)
                .map_err(|_| format!("Invalid hex color: '{s}'"))?;
            let g = u8::from_str_radix(&s[3..5], 16)
                .map_err(|_| format!("Invalid hex color: '{s}'"))?;
            let b = u8::from_str_radix(&s[5..7], 16)
                .map_err(|_| format!("Invalid hex color: '{s}'"))?;
            Ok(ColorArg(Color::Rgb { r, g, b }))
        }
        s if s.starts_with("rgb:") => {
            let parts: Vec<&str> = s[4..].split(',').collect();
            if parts.len() != 3 {
                return Err(format!("Expected rgb:R,G,B, got: '{s}'"));
            }
            let [r, g, b] = [parts[0], parts[1], parts[2]].map(|p| {
                p.trim()
                    .parse::<u8>()
                    .map_err(|_| format!("Invalid RGB component: '{p}'"))
            });
            Ok(ColorArg(Color::Rgb {
                r: r?,
                g: g?,
                b: b?,
            }))
        }
        _ => Err(format!(
            "Unknown color '{s}'. Try: red, blue, #ff0000, rgb:255,0,0, ansi:200"
        )),
    }
}

impl fmt::Display for ColorArg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Color::Black => write!(f, "black"),
            Color::Red => write!(f, "red"),
            Color::Green => write!(f, "green"),
            Color::Yellow => write!(f, "yellow"),
            Color::Blue => write!(f, "blue"),
            Color::Magenta => write!(f, "magenta"),
            Color::Cyan => write!(f, "cyan"),
            Color::White => write!(f, "white"),
            Color::Grey => write!(f, "grey"),
            Color::DarkRed => write!(f, "darkred"),
            Color::DarkGreen => write!(f, "darkgreen"),
            Color::DarkYellow => write!(f, "darkyellow"),
            Color::DarkBlue => write!(f, "darkblue"),
            Color::DarkMagenta => write!(f, "darkmagenta"),
            Color::DarkCyan => write!(f, "darkcyan"),
            Color::DarkGrey => write!(f, "darkgrey"),
            Color::Rgb { r, g, b } => write!(f, "#{r:02x}{g:02x}{b:02x}"),
            Color::AnsiValue(n) => write!(f, "ansi:{n}"),
            _ => write!(f, "{self:?}"),
        }
    }
}

impl From<ColorArg> for Color {
    fn from(c: ColorArg) -> Self {
        c.0
    }
}
