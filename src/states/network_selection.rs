use ratatui::{
    Frame,
    widgets::{Block, Borders, List, ListItem, Paragraph},
    layout::{Layout, Constraint, Direction, Alignment},
    style::{Color, Style, Modifier},
    text::{Line, Span},
};
use crossterm::event::{KeyCode, KeyEvent};

use crate::{app::AppContext, garden_api::{orderbook::Orderbook, quote::Quote}};
use super::{State, StateType};

pub struct NetworkSelectionState;

impl NetworkSelectionState {
    pub fn new() -> Self {
        NetworkSelectionState
    }
    
    pub fn next(context: &mut AppContext) {
        let i = match context.network_list_state.selected() {
            Some(i) => {
                if i >= context.networks.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        context.network_list_state.select(Some(i));
    }
    
    pub fn previous(context: &mut AppContext) {
        let i = match context.network_list_state.selected() {
            Some(i) => {
                if i == 0 {
                    context.networks.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        context.network_list_state.select(Some(i));
    }
    
    pub fn select_network(context: &mut AppContext) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(api_urls) = &context.api_urls {
            if let Some(selected) = context.network_list_state.selected() {
                let network_key = context.networks[selected].to_lowercase();
                context.selected_network = Some(context.networks[selected].to_string());
                
                if let Some(network_config) = api_urls.get(&network_key) {
                    // Parse the network config object
                    if let Some(evm_url) = network_config.get("evm_relayer_url").and_then(|v| v.as_str()) {
                        if let Some(quote_url) = network_config.get("quote_server_url").and_then(|v| v.as_str()) {
                            if let Some(vb_url) = network_config.get("virtual_balance_server_url").and_then(|v| v.as_str()) {
                                context.selected_network_urls = Some(crate::app::NetworkUrls {
                                    evm_relayer_url: evm_url.to_string(),
                                    quote_server_url: quote_url.to_string(),
                                    virtual_balance_server_url: vb_url.to_string(),
                                });
                                
                                let http_client = reqwest::blocking::Client::new();
                                let quote = Quote::new(
                                    http_client.clone(),
                                    quote_url.to_string()
                                );
                                context.quote = Some(quote);
                                
                                context.orderbook = Some(
                                    Orderbook::new(
                                        http_client.clone(),
                                        context.selected_network_urls.as_ref().unwrap().evm_relayer_url.clone(),
                                        "ACOG8te1sEI6OrR_HNqSL_Y7_JzOZTqYIKVk3wqjxCjH8G_3uLOIlnntJPJXQEJqwmFQuuA_g7FqQhNZBPtFPEflQKazrDK7_24c".to_string()
                                ));
                                return Ok(());
                            }
                        }
                    }
                }
                
                return Err(format!("Could not find valid URLs for network: {}", network_key).into());
            }
        }
        
        Err("API URLs not loaded".into())
    }
}

impl State for NetworkSelectionState {
    fn draw(&self, frame: &mut Frame, context: &mut AppContext) {
        let size = frame.area();
        
        // Create common elements
        let title_span = Span::styled(
            "Garden-TUI 0.0.1", 
            Style::default().fg(Color::LightRed).add_modifier(Modifier::BOLD)
        );
        let title_line = Line::from(vec![title_span]);
        
        let title_block = Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White));
            
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
        frame.render_widget(
            Paragraph::new(vec![title_line])
                .block(title_block)
                .alignment(Alignment::Center),
            chunks[0],
        );
        
        // Network selector
        let network_items: Vec<ListItem> = context.networks
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
        
        frame.render_stateful_widget(network_list, chunks[2], &mut context.network_list_state);
        
        // Instructions
        let instructions_spans = vec![
            Span::styled("↑↓: Navigate | ", Style::default().fg(Color::Red)),
            Span::styled("Enter: Select | ", Style::default().fg(Color::Red)),
            Span::styled("q: Quit", Style::default().fg(Color::Red)),
        ];
        
        frame.render_widget(
            Paragraph::new(vec![Line::from(instructions_spans)])
                .alignment(Alignment::Center),
            chunks[3],
        );
    }
    
    fn handle_key(&mut self, key: KeyEvent, context: &mut AppContext) -> Option<StateType> {
        match key.code {
            KeyCode::Char('q') => Some(StateType::Quit),
            KeyCode::Down => {
                Self::next(context);
                None
            },
            KeyCode::Up => {
                Self::previous(context);
                None
            },
            KeyCode::Enter => {
                match Self::select_network(context) {
                    Ok(_) => Some(StateType::NetworkInformation),
                    Err(e) => {
                        // Handle error (in a real app, you might want to show this in the UI)
                        eprintln!("Error: {}", e);
                        None
                    }
                }
            },
            _ => None,
        }
    }
}