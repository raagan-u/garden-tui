use crossterm::event::{KeyEvent, KeyCode};
use crossterm::style::style;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use ratatui::prelude::*;


use crate::context::AppContext;
use crate::service::garden::quote::generate_secret;
use crate::service::garden::types::Order;
use crate::service::garden::types::OrderInputData;

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
        
        let title_span = match &context.order.current_strategy {
            Some(strategy) => {
                match context.api.quote.strategy_readable(strategy) {
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
                Constraint::Length(3), //title
                Constraint::Length(1),
                Constraint::Length(4), //address
                Constraint::Length(1),
                Constraint::Length(3), //in
                Constraint::Length(3), // out
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
        
        let address_block = Block::default()
            .title("Your Addresses")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White));
       
        let addresses = format!("EVM: {}\nBTC: {}",context.wallet.signer.address().to_string(), context.wallet.btc_address);
       
        frame.render_widget(
            Paragraph::new(addresses)
                .block(address_block)
                .alignment(Alignment::Left),
            chunks[2],
        );
            
        
        let input_block = Block::default()
            .title("In Amount")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White));
        
        frame.render_widget(
            Paragraph::new(self.input_value.clone())
                .block(input_block)
                .alignment(Alignment::Left),
            chunks[4],
        );
            
        let output_block = Block::default()
            .title("Out Amount")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White));
            
        
        frame.render_widget(
            Paragraph::new(self.quote_price.clone())
                .block(output_block)
                .alignment(Alignment::Left),
            chunks[5],
        );
            
        if self.input_focused {
            frame.set_cursor_position(
                Position::new(chunks[4].x + 1 + self.input_value.len() as u16, chunks[4].y + 1)
            );
        }
            
        let instructions_spans = vec![
            Span::styled("q: Quit | ", Style::default().fg(Color::Red)),
            Span::styled("b: Strategy Selection | ", Style::default().fg(Color::Red)),
            Span::styled("g: Get Quote | ", Style::default().fg(Color::Red)),
            Span::styled("s: SWAP | ", Style::default().fg(Color::Green)),
            Span::styled("i: Toggle Input Focus", Style::default().fg(Color::Red)),
        ];
            
        frame.render_widget(
            Paragraph::new(vec![Line::from(instructions_spans)])
                .alignment(Alignment::Center),
            chunks[6],
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
            KeyCode::Char('s') => {
                if self.quote_price.is_empty() || self.input_value.is_empty(){
                    self.quote_price = "please ensure to get quote price".to_string();
                    return None
                }
                if let Some(current_strategy) = &context.order.current_strategy {
                    if let Some(strategy) = context.api.quote.strategies_map.get(current_strategy) {
                        let in_amount = self.input_value.parse::<u64>().unwrap();
                        let out_amount = self.quote_price.parse::<u64>().unwrap();
                        let (init_src_add, init_dest_addr, btc_opt_recp ) = if strategy.source_chain.contains("bitcoin") {
                            (context.wallet.btc_xpubkey.to_string(), context.wallet.signer.address().to_string(), None)
                        } else if strategy.dest_chain.contains("bitcoin") {
                            
                            (context.wallet.signer.address().to_string(), context.wallet.btc_xpubkey.to_string(), Some(context.wallet.btc_address.clone()))
                        } else {
                            (context.wallet.signer.address().to_string(), context.wallet.signer.address().to_string(), None)
                        };
                        
                        let (secret, secret_hash) = generate_secret().unwrap();
                        let _order = Order::new(OrderInputData{
                            initiator_source_address: init_src_add,
                            initiator_dest_address: init_dest_addr,
                            in_amount,
                            out_amount, 
                            secret_hash: hex::encode(secret_hash),
                            strategy: strategy.clone(),
                            btc_opt_recepient: btc_opt_recp
                        });
                        
                        
                        let attested_order = context.api.quote.get_attested_quote(_order).expect("error getting attested quote");
                        context.order.current_order = Some(attested_order);
                        context.order.secret = secret
                    }
                }
                Some(StateType::OrderInformation)
            },
            KeyCode::Char('g') => {
                if self.input_value.is_empty(){
                    self.quote_price = "please enter a valid input amount".to_string();
                    return None
                }
                if let Some(strategy) = &context.order.current_strategy {
                    let details = match context.api.quote.strategies_map.get(strategy) {
                        Some(details) => details,
                        None => return None,
                    };
                   
                    let order_pair = format!("{}:{}::{}:{}", details.source_chain, details.source_asset.asset, details.dest_chain, details.dest_asset.asset);
                    if let Ok(price) = context.api.quote.get_price(&order_pair, &self.input_value) {
                        self.quote_price = price.trim_matches('"').to_string();
                        return None
                    }
                }
                None
            }
            KeyCode::Char(c) => {
                if c.is_ascii_digit(){
                    self.handle_input(c);
                }
                None
            },
            KeyCode::Backspace => {
                self.handle_backspace();
                None
            },
            _ => None,
        }
    }
}