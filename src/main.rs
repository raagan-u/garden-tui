// src/main.rs
use std::{env, error::Error};
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
use app::App;

fn main() -> Result<(), Box<dyn Error>> {
    let eth_priv_key = env::var("PRIV_KEY").expect("please provide a valid PRIV_KEY in env");
    let btc_priv_key = env::var("BTC_PRIV_KEY").unwrap_or(eth_priv_key.clone());
    
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and load API URLs
    let mut app = App::new(&eth_priv_key);
    app.load_config()?;

    // Main loop
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