use std::error::Error;
use std::panic;
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
pub mod service;
mod context;
mod config;
use app::App;


fn restore_terminal() -> Result<(), Box<dyn Error>> {
    disable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(
        stdout,
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let matches = Command::new("garden-tui")
        .version("1.0")
        .about("cross chain swaps from terminal")
        .args([
            Arg::new("network")
                .short('n')
                .long("network")
                .value_name("NETWORK")
                .help("Specifies the network to use (testnet, localnet, mainnet)")
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
    
    
    let network_name = matches.get_one::<String>("network").expect("error retrieving network");
    let config_file_path = matches.get_one::<String>("config").expect("Config file path is required");
    
    // Set up panic hook before touching the terminal
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = restore_terminal();
        // call the original panic handler after restoring terminal
        original_hook(panic_info);
    }));
    
    // Initialize terminal
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    
    // Run the app inside a result-returning function for clean error handling
    let run_app_result = (|| -> Result<Option<String>, Box<dyn Error>> {
        let config = config::Config::from_file(config_file_path)?;
        let mut app = App::new(network_name, config);
        
        while !app.should_quit {
            terminal.draw(|f| app.draw(f))?;
            if let Event::Key(key) = event::read()? {
                app.handle_key(key);
            }
        }
        
        Ok(app.get_final_message())
    })();
    
    
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    
    
    match run_app_result {
        Ok(final_message) => {
            if let Some(message) = final_message {
                println!("{}", message);
            }
            Ok(())
        },
        Err(err) => {
            eprintln!("Application error: {}", err);
            Err(err)
        }
    }
}