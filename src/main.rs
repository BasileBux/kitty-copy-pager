use clap::Parser;
use crossterm::{
    cursor::MoveTo,
    event::{Event, poll, read},
    execute,
    terminal::{
        Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
        enable_raw_mode,
    },
};
use kitty_copy::{scrollback::ScrollbackBuffer};
use kitty_copy::settings::*;
use std::io::{self, Write, stdout};
use std::time::Duration;

use log::*;
use simplelog::*;

const LOGGING_ENABLED: bool = false;

const INPUT_POLLING_RATE: u64 = 100;

fn main() -> io::Result<()> {
    let args = Args::parse();
    let settings = Settings::from_args(args);
    let mut sb = ScrollbackBuffer::new(settings)?;

    if LOGGING_ENABLED {
        WriteLogger::init(
            LevelFilter::Debug,
            Config::default(),
            std::fs::File::create("debug.log").unwrap(),
        )
        .unwrap();
    }

    execute!(stdout(), EnterAlternateScreen, Clear(ClearType::All))?;
    enable_raw_mode()?;
    execute!(stdout(), MoveTo(0, 0))?;
    stdout().flush()?;

    sb.draw()?;
    sb.draw_status_line()?;

    loop {
        if poll(Duration::from_millis(INPUT_POLLING_RATE))? {
            let event = read()?;
            match event {
                Event::Key(e) => {
                    let quit = sb.handle_key_event(e)?;
                    if quit {
                        break;
                    }
                }
                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;
    Ok(())
}
