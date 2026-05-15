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
#[cfg(feature = "dhat-heap")]
use dhat;
use kitty_copy_pager::pager::Pager;
use kitty_copy_pager::settings::*;
use std::io::{self, Write, stdout};
use std::time::Duration;

use log::*;
use simplelog::*;

const LOGGING_ENABLED: bool = false;

const INPUT_POLLING_RATE: u64 = 100;

#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

fn main() -> io::Result<()> {
    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();
    let args = Args::parse();
    let settings = Settings::from_args(args);
    let mut sb = Pager::new(settings)?;

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
            if let Event::Key(e) = event {
                let quit = sb.handle_key_event(e)?;
                if quit {
                    break;
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;
    Ok(())
}
