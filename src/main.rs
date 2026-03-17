use crossterm::{
    cursor::MoveTo,
    event::{Event, poll, read},
    execute,
    terminal::{
        Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
        enable_raw_mode,
    },
};
use kitty_copy::scrollback::ScrollbackBuffer;
use std::io::{self, Write, stdout};
use std::time::Duration;

use log::*;
use simplelog::*;

fn main() -> io::Result<()> {
    let mut sb = ScrollbackBuffer::new()?;

    WriteLogger::init(
        LevelFilter::Debug,
        Config::default(),
        std::fs::File::create("debug.log").unwrap(),
    )
    .unwrap();

    execute!(stdout(), EnterAlternateScreen, Clear(ClearType::All))?;
    enable_raw_mode()?;
    execute!(stdout(), MoveTo(0, 0))?;
    stdout().flush()?;

    sb.draw()?;

    loop {
        if poll(Duration::from_millis(100))? {
            let event = read()?;
            match event {
                Event::Key(e) => {
                    if e.code == crossterm::event::KeyCode::Char('q') {
                        break;
                    }
                    sb.handle_key_event(e)?;
                }
                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;
    Ok(())
}
