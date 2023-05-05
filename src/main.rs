mod app;
mod ui;

use std::{
    env, fs, io,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use app::App;
use clap::Parser;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use tui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};

#[derive(Parser, Debug)]
#[command(author="Vincent Udén", version=env!("CARGO_PKG_VERSION"), about="A terminal file manager")]
struct Args {
    #[arg(long, help = "Output the last visited directory to a given file")]
    choose_dir: Option<PathBuf>,
}

fn main() -> Result<(), io::Error> {
    let args = Args::parse();

    enable_raw_mode()?;

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let p = env::current_dir().unwrap_or(Path::new("/").to_path_buf());
    let mut app = App::new(String::from("File Manager"), &p);
    app.init();
    run_app(&mut terminal, &mut app, Duration::from_millis(100))?;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture,
    )?;
    terminal.show_cursor()?;

    match args.choose_dir {
        Some(p) => {
            fs::write(p.as_path(), app.current_dir.to_str().unwrap_or("./"))?;
        }
        None => {}
    }

    Ok(())
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    tick_rate: Duration,
) -> io::Result<()> {
    let mut last_tick = Instant::now();

    loop {
        app.draw(terminal)?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = crossterm::event::read()? {
                match key.code {
                    crossterm::event::KeyCode::Char(_) => {
                        app.on_key(key);
                    }
                    /* app.on_key used to take a character instead of a KeyEvent,
                     * thus, helper function were required for Key presses not
                     * corresponding to a char. Is there any benefit of keeping
                     * these as separate functions?
                     */
                    crossterm::event::KeyCode::Esc => {
                        app.on_esc();
                    }
                    crossterm::event::KeyCode::Enter => {
                        app.on_enter();
                    }
                    crossterm::event::KeyCode::Backspace => {
                        app.on_backspace();
                    }
                    crossterm::event::KeyCode::Up => {
                        app.on_up();
                    }
                    crossterm::event::KeyCode::Down => {
                        app.on_down();
                    }
                    _ => {}
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.on_tick();
            last_tick = Instant::now();
        }

        if app.should_quit {
            app.tear_down();
            return Ok(());
        }
    }
}
