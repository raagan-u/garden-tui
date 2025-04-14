use std::error::Error;
use clap::{Arg, Command};
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
    let matches = Command::new("garden-tui")
        .version("1.0")
        .about("cross chain swaps from terminal")
        .args([
            Arg::new("network")
                .short('n')
                .long("network")
                .value_name("NETWORK")
                .help("Specifies the network to use (testnet, localnet)")
                .default_value("localnet")
                .required(false),
            Arg::new("config")
                .short('c')
                .long("conifg")
                .value_name("CONFIG")
                .help("path to config file")
                .required(true),
        ])
        .get_matches();

    // Get the network from command-line arguments
    let network_name = matches.get_one::<String>("network").unwrap();
    let config_file_path = matches.get_one::<String>("config").unwrap();
    
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    
    let config = config::Config::from_file(config_file_path)?;
    let mut app = App::new(network_name, config);

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