use std::error::Error;
use app::{App, AppState};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
    widgets::{Block, Borders, List, ListItem, Paragraph},
    layout::{Layout, Constraint, Direction, Alignment},
    style::{Color, Style, Modifier},
    text::{Line, Span},
};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

mod garden_api;
mod htlc;
mod app;

fn main() -> Result<(), Box<dyn Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and load API URLs
    let mut app = App::new();
    app.load_api_urls()?;

    // Main loop
    loop {
        terminal.draw(|f| {
            let size = f.area();
            
            // Create common elements
            let title_span = Span::styled(
                "Garden-TUI 0.0.1", 
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
            );
            let title_line = Line::from(vec![title_span]);
            
            let title_block = Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::White));
            
            match app.state {
                AppState::NetworkSelection => {
                    // Create layout for network selection
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .margin(2)
                        .constraints([
                            Constraint::Length(3),  // Title
                            Constraint::Length(1),  // Spacer
                            Constraint::Length(5),  // Network List
                            Constraint::Min(0),     // Remaining space
                        ].as_ref())
                        .split(size);
                    
                    // Title
                    f.render_widget(
                        Paragraph::new(vec![title_line])
                            .block(title_block)
                            .alignment(Alignment::Center),
                        chunks[0],
                    );
                    
                    // Network selector
                    let network_items: Vec<ListItem> = app.networks
                        .iter()
                        .map(|n| {
                            let span = Span::styled(*n, Style::default().fg(Color::White));
                            ListItem::new(Line::from(vec![span]))
                        })
                        .collect();
                    
                    let network_list = List::new(network_items)
                        .block(Block::default().title("Select Network").borders(Borders::ALL))
                        .highlight_style(Style::default().fg(Color::Black).bg(Color::LightGreen))
                        .highlight_symbol("> ");
                    
                    f.render_stateful_widget(network_list, chunks[2], &mut app.network_list_state);
                    
                    // Instructions
                    let instructions_spans = vec![
                        Span::styled("↑↓: Navigate | ", Style::default().fg(Color::DarkGray)),
                        Span::styled("Enter: Select | ", Style::default().fg(Color::DarkGray)),
                        Span::styled("q: Quit", Style::default().fg(Color::DarkGray)),
                    ];
                    
                    f.render_widget(
                        Paragraph::new(vec![Line::from(instructions_spans)])
                            .alignment(Alignment::Center),
                        chunks[3],
                    );
                },
                AppState::NetworkInformation => {
                    // Create layout for network information display
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .margin(2)
                        .constraints([
                            Constraint::Length(3),   // Title
                            Constraint::Length(1),   // Spacer
                            Constraint::Length(10),  // Network Info
                            Constraint::Min(0),      // Remaining space
                        ].as_ref())
                        .split(size);
                    
                    // Title
                    f.render_widget(
                        Paragraph::new(vec![title_line])
                            .block(title_block)
                            .alignment(Alignment::Center),
                        chunks[0],
                    );
                    
                    // Network Information
                    if let (Some(network), Some(urls)) = (&app.selected_network, &app.selected_network_urls) {
                        let info_text = vec![
                            Line::from(vec![
                                Span::styled(format!("Selected Network: "), 
                                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                                Span::styled(network, Style::default().fg(Color::White))
                            ]),
                            Line::from(vec![Span::raw("")]),
                            Line::from(vec![
                                Span::styled("EVM Relayer URL: ", 
                                    Style::default().fg(Color::Yellow)),
                                Span::styled(&urls.evm_relayer_url, Style::default().fg(Color::White))
                            ]),
                            Line::from(vec![
                                Span::styled("Quote Server URL: ", 
                                    Style::default().fg(Color::Yellow)),
                                Span::styled(&urls.quote_server_url, Style::default().fg(Color::White))
                            ]),
                            Line::from(vec![
                                Span::styled("Virtual Balance Server URL: ", 
                                    Style::default().fg(Color::Yellow)),
                                Span::styled(&urls.virtual_balance_server_url, Style::default().fg(Color::White))
                            ]),
                        ];
                        
                        let info_block = Block::default()
                            .title("Network Information")
                            .borders(Borders::ALL);
                        
                        f.render_widget(
                            Paragraph::new(info_text)
                                .block(info_block)
                                .alignment(Alignment::Left),
                            chunks[2],
                        );
                    }
                    
                    // Instructions
                    let instructions_spans = vec![
                        Span::styled("b: Back to selection | ", Style::default().fg(Color::DarkGray)),
                        Span::styled("c: Continue to app | ", Style::default().fg(Color::DarkGray)),
                        Span::styled("q: Quit", Style::default().fg(Color::DarkGray)),
                    ];
                    
                    f.render_widget(
                        Paragraph::new(vec![Line::from(instructions_spans)])
                            .alignment(Alignment::Center),
                        chunks[3],
                    );
                }
            }
        })?;

        // Handle input
        if let Event::Key(key) = event::read()? {
            match app.state {
                AppState::NetworkSelection => {
                    match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Down => app.next(),
                        KeyCode::Up => app.previous(),
                        KeyCode::Enter => {
                            app.select_network()?;
                        },
                        _ => {}
                    }
                },
                AppState::NetworkInformation => {
                    match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Char('b') => app.back_to_selection(),
                        KeyCode::Char('c') => {
                            // Here you would transition to your main app
                            // For now, we'll just print the details and exit
                            if let (Some(network), Some(urls)) = (&app.selected_network, &app.selected_network_urls) {
                                disable_raw_mode()?;
                                execute!(
                                    terminal.backend_mut(),
                                    LeaveAlternateScreen,
                                    DisableMouseCapture
                                )?;
                                terminal.show_cursor()?;
                                
                                println!("Selected network: {}", network);
                                println!("EVM Relayer URL: {}", urls.evm_relayer_url);
                                println!("Quote Server URL: {}", urls.quote_server_url);
                                println!("Virtual Balance Server URL: {}", urls.virtual_balance_server_url);
                                return Ok(());
                            }
                        },
                        _ => {}
                    }
                }
            }
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

    Ok(())
}