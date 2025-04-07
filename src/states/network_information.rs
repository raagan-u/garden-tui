// src/states/network_information.rs
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout}, style::{Color, Modifier, Style}, text::{Line, Span}, widgets::{Block, Borders, List, ListItem, Paragraph}, Frame
};
use crossterm::event::{KeyCode, KeyEvent};

use crate::app::AppContext;
use super::{strategy_selector::StrategySelector, State, StateType};

pub struct NetworkInformationState;


impl NetworkInformationState {
    pub fn new() -> Self {
        NetworkInformationState
    }
}

impl State for NetworkInformationState {
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
            
        // Create layout with more explicit constraints
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Length(3),   // Title
                Constraint::Length(5),   // Network Info - increased height
                Constraint::Length(10),  // Strategy Selector
                Constraint::Length(3),   // Selected Strategy (if any)
                Constraint::Min(0),      // Instructions
            ].as_ref())
            .split(size);
        
        // Title
        frame.render_widget(
            Paragraph::new(vec![title_line])
                .block(title_block)
                .alignment(Alignment::Center),
            chunks[0],
        );
        
        // Network Information with Strategy Selector
        if let (Some(network), Some(urls)) = (&context.selected_network, &context.selected_network_urls) {
            let quote = match context.quote.as_mut() {
                Some(q) => q,
                None => {
                    eprintln!("Quote is None");
                    return;
                }
            };
            
            // Network info
            let info_text = vec![
                Line::from(vec![
                    Span::styled(format!("Selected Network: "), 
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                    Span::styled(network, Style::default().fg(Color::White))
                ]),
                Line::from(vec![
                    Span::styled("Quote Server URL: ", 
                        Style::default().fg(Color::Yellow)),
                    Span::styled(&urls.quote_server_url, Style::default().fg(Color::White))
                ]),
            ];
            
            let info_block = Block::default()
                .title("Network Information")
                .borders(Borders::ALL);
            
            frame.render_widget(
                Paragraph::new(info_text)
                    .block(info_block)
                    .alignment(Alignment::Left),
                chunks[1],
            );
            
            // Check and initialize strategy selector
            if context.strategy_selector.is_none() {
                quote.load_strategies().unwrap();
                if let Some(strategies) = quote.strategies_map.as_ref() {
                    if !strategies.is_empty() {
                        context.strategy_selector = Some(StrategySelector::new(strategies));
                    } else {
                        eprintln!("Strategies map is empty");
                    }
                } else {
                    eprintln!("Strategies map is None");
                }
            }
            
            // Render the strategy selector
            if let Some(selector) = context.strategy_selector.as_mut() {
                if !selector.strategies.is_empty() {
                    let items: Vec<ListItem> = selector.strategies
                        .iter()
                        .map(|(id, strategy)| {
                            ListItem::new(format!("{}: {} to {}", 
                                id, 
                                strategy.source_chain, 
                                strategy.dest_chain))
                        })
                        .collect();
                    
                    let list = List::new(items)
                        .block(Block::default().title("Select Strategy").borders(Borders::ALL))
                        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                        .highlight_symbol("> ");
                    
                    frame.render_stateful_widget(list, chunks[2], &mut selector.state);
                    
                    // Show currently selected strategy if any
                    if let Some(strategy_id) = context.current_strategy.as_ref() {
                        let selected_text = vec![
                            Line::from(vec![
                                Span::styled("Selected Strategy: ", 
                                    Style::default().fg(Color::Yellow)),
                                Span::styled(strategy_id, Style::default().fg(Color::Green))
                            ])
                        ];
                        
                        frame.render_widget(
                            Paragraph::new(selected_text)
                                .block(Block::default().borders(Borders::ALL)),
                            chunks[3],
                        );
                    }
                } else {
                    // Show message if no strategies available
                    frame.render_widget(
                        Paragraph::new("No strategies available")
                            .block(Block::default().title("Select Strategy").borders(Borders::ALL))
                            .alignment(Alignment::Center),
                        chunks[2],
                    );
                }
            } else {
                // Show message if selector not initialized
                frame.render_widget(
                    Paragraph::new("Loading strategies...")
                        .block(Block::default().title("Select Strategy").borders(Borders::ALL))
                        .alignment(Alignment::Center),
                    chunks[2],
                );
            }
        } else {
            eprintln!("Network or URLs not selected");
        }
        
        // Instructions
        let instructions_spans = vec![
            Span::styled("↑/↓: Navigate | ", Style::default().fg(Color::Red)),
            Span::styled("Enter: Select Strategy | ", Style::default().fg(Color::Red)),
            Span::styled("b: Back | ", Style::default().fg(Color::Red)),
            Span::styled("q: Quit", Style::default().fg(Color::Red)),
        ];
        
        frame.render_widget(
            Paragraph::new(vec![Line::from(instructions_spans)])
                .alignment(Alignment::Center),
            chunks[4],
        );
    }
    
    fn handle_key(&mut self, key: KeyEvent, context: &mut AppContext) -> Option<StateType> {
        match key.code {
            KeyCode::Char('q') => Some(StateType::Quit),
            KeyCode::Char('b') => Some(StateType::NetworkSelection),
            KeyCode::Up => {
                if let Some(selector) = context.strategy_selector.as_mut() {
                    selector.previous();
                }
                None
            },
            KeyCode::Down => {
                if let Some(selector) = context.strategy_selector.as_mut() {
                    selector.next();
                }
                None
            },
            KeyCode::Enter => {
                if let Some(selector) = &context.strategy_selector {
                    if let Some((id, _)) = selector.selected_strategy() {
                        context.current_strategy = Some(id.clone());
                    }
                }
                Some(StateType::Swapinformation)
            },
            _ => None,
        }
    }
}