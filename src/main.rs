// src/main.rs
use std::error::Error;
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};


mod app;
mod ui;
mod service;
mod context;
mod config;
use app::App;

fn main() -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    
    let config = config::Config::from_file("config.json")?;
    let mut app = App::new(config.get_network("localnet")?.clone());

    while !app.should_quit {
        terminal.draw(|f| app.draw(f))?;
        if let Event::Key(key) = event::read()? {
            app.handle_key(key);
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    // Display final information if needed
    if let Some(final_message) = app.get_final_message() {
        println!("{}", final_message);
    }

    Ok(())
}