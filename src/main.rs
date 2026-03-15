use crossterm::{
    cursor::MoveTo,
    event::{Event, poll, read},
    execute,
    terminal::{
        Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
        enable_raw_mode,
    },
};
use std::io::{self, Write, stdin, stdout};
use std::time::Duration;

const PROMPT_CURSOR_OFFSET: usize = 2;

pub struct ScrollbackBuffer {
    lines: Vec<String>,
    pos_x: usize,
    pos_y: usize,
    term_width: usize,
    term_height: usize,
    viewport_start: usize,
    viewport_end: usize,
}

impl ScrollbackBuffer {
    pub fn new() -> io::Result<Self> {
        let mut lines = Vec::<String>::new();
        for line in stdin().lines() {
            lines.push(line?); // Doesn't keep the trailing newline
        }
        let (term_width, term_height) = crossterm::terminal::size()?;

        // The scrollback may contain empty lines at the end
        // TODO: rework to remove unwated empty lines from the buffer
        // This will demand to rework the printing relative to the viewport and thus
        // the viewport management itself
        let mut pos_y = lines.len().saturating_sub(1);
        while pos_y > 0 && lines[pos_y].is_empty() {
            pos_y -= 1;
        }

        Ok(Self {
            pos_x: lines.last().map(|l| l.len()).unwrap_or(0) + PROMPT_CURSOR_OFFSET,
            pos_y: pos_y.saturating_sub(0),

            term_width: term_width as usize,
            term_height: term_height as usize,

            viewport_start: lines.len().saturating_sub(term_height as usize),
            viewport_end: lines.len() - 1,

            lines: lines,
        })
    }

    pub fn draw(&self) -> io::Result<()> {
        // TODO: render relative to the cursor position and viewport
        for (i, line) in self.lines.iter().enumerate() {
            execute!(stdout(), MoveTo(0, i as u16))?;
            print!("{}", line); // TODO: optimize to build a buffer to print at once
        }
        execute!(stdout(), MoveTo(self.pos_x as u16, self.pos_y as u16))?;
        stdout().flush()?;
        Ok(())
    }
}

fn main() -> io::Result<()> {
    let mut scrollback = ScrollbackBuffer::new()?;

    execute!(stdout(), EnterAlternateScreen, Clear(ClearType::All))?;
    enable_raw_mode()?;
    execute!(stdout(), MoveTo(0, 0))?;
    stdout().flush()?;

    scrollback.draw()?;

    loop {
        if poll(Duration::from_millis(100))? {
            let event = read()?;
            match event {
                Event::Key(e) => {
                    if e.code == crossterm::event::KeyCode::Char('q') {
                        break;
                    }
                    // TODO: Implement simple movment vim keys (h, j, k, l)
                }
                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;
    Ok(())
}
