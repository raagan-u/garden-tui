use crossterm::event::{KeyEvent, KeyCode};
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use ratatui::prelude::*;

use crate::app::AppContext;
use crate::garden_api::types::Order;
use crate::garden_api::types::OrderInputData;

use super::{State, StateType};

pub struct SwapDashboardState {
    input_value: String,
    input_focused: bool,
    quote_price: String,
}

impl SwapDashboardState {
    pub fn new() -> Self {
        SwapDashboardState {
            input_value: "".to_string(),
            input_focused: false,
            quote_price: "".to_string(),
        }
    }
    
    pub fn toggle_input_focus(&mut self) {
            self.input_focused = !self.input_focused;
        }
        
    pub fn handle_input(&mut self, key: char) {
        if self.input_focused {
            self.input_value.push(key);
        }
    }
    
    pub fn handle_backspace(&mut self) {
        if self.input_focused && !self.input_value.is_empty() {
            self.input_value.pop();
        }
    }
}
impl State for SwapDashboardState {
    fn draw(&self, frame: &mut Frame, context: &mut AppContext){
        let size = frame.area();
        
        let title_span = match (&context.quote, &context.current_strategy) {
            (Some(quote), Some(strategy)) => {
                match quote.strategy_readable(strategy) {
                    Ok(order_pair) => vec![
                        Span::styled("Current Strategy Selected  ", 
                            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                        Span::styled(order_pair, 
                            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
                    ],
                    Err(_) => vec![
                        Span::styled("Current Strategy Selected  ", 
                            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                        Span::styled("(Unknown Order Pair)", 
                            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                    ]
                }
            },
            _ => vec![
                Span::styled("No Strategy Selected", 
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
            ]
        };
        let title_line = Line::from(title_span);
        let title_block = Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White));
            
        // Create layout for network selection
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Length(3),  // Title
                Constraint::Length(1),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(0)
            ].as_ref())
            .split(size);
        
        // Title
        frame.render_widget(
            Paragraph::new(vec![title_line])
                .block(title_block)
                .alignment(Alignment::Center),
            chunks[0],
        );
        
        let input_block = Block::default()
                .title("In Amount")
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::White));
            
        frame.render_widget(
            Paragraph::new(self.input_value.clone())
                .block(input_block)
                .alignment(Alignment::Left),
            chunks[2],
        );
            
        let output_block = Block::default()
            .title("Out Amount")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White));
            
        // Fix: Don't use render_stateful_widget for a string
        frame.render_widget(
            Paragraph::new(self.quote_price.clone())
                .block(output_block)
                .alignment(Alignment::Left),
            chunks[3],
        );
            
        if self.input_focused {
            frame.set_cursor(
                chunks[2].x + 1 + self.input_value.len() as u16,
                chunks[2].y + 1,
            );
        }
            
        let instructions_spans = vec![
            Span::styled("q: Quit | ", Style::default().fg(Color::Red)),
            Span::styled("b: Strategy Selection | ", Style::default().fg(Color::Red)),
            Span::styled("s: SWAP | ", Style::default().fg(Color::Green)),
            Span::styled("i: Toggle Input Focus", Style::default().fg(Color::Red)),
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
            KeyCode::Char('b') => Some(StateType::NetworkInformation), 
            KeyCode::Char('i') => {
                self.toggle_input_focus();
                None
            },
            KeyCode::Char('g') => {
                if let (Some(quote), Some(current_strategy)) = (&context.quote, &context.current_strategy) {
                    if let Some(strategy) = quote.strategies_map.as_ref()
                        .and_then(|map| map.get(current_strategy).cloned()) {

                        let in_amount = self.input_value.parse::<u64>().unwrap();
                        let out_amount = self.quote_price.parse::<u64>().unwrap();

                        let _order = Order::new(OrderInputData{
                            initiator_source_address: context.signer.address().to_string(),
                            in_amount,
                            out_amount, 
                            secret_hash: "a8c26c709cc11d0102f245bc8d868e71490adcf04c810ae9550a5da03cf94139".to_string(),
                            strategy,
                            btc_opt_recepient: None
                        });
                        
                        let attested_order = quote.get_attested_quote(_order).expect("error getting attested quote");
                        context.current_order = Some(attested_order);
                    }
                }
                None
            },
            KeyCode::Char('s') => None,
            KeyCode::Char(c) => {
                if c.is_ascii_digit(){
                    self.handle_input(c);
                }
                None
            },
            KeyCode::Enter => {
                if let (Some(quote), Some(strategy)) = (&context.quote, &context.current_strategy) {
                    if let Ok(order_pair) = quote.strategy_to_order_pair(strategy) {
                        if let Ok(price) = quote.get_price(&order_pair, &self.input_value) {
                            self.quote_price = price.trim_matches('"').to_string();
                        }
                    }
                }
                None
            }
            KeyCode::Backspace => {
                self.handle_backspace();
                None
            },
            _ => None,
        }
    }
}