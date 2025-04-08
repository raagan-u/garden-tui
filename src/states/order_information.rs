use std::str::FromStr;
use std::thread::sleep;
use std::time::Duration;

use alloy::providers::ProviderBuilder;
use crossterm::event::{KeyEvent, KeyCode};
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use ratatui::prelude::*;
use reqwest::Url;

use crate::app::AppContext;
use crate::garden_api::types::InitiateRequest;
use crate::htlc::utils::init_and_get_sig;

use super::{State, StateType};

pub struct OrderDashboardState {
    pub order_id: String,
    pub status: Option<String>,
}

impl OrderDashboardState {
    pub fn new() -> Self {
        OrderDashboardState {
            order_id: "Press 's' to create-order".to_string(),
            status: None,
        }
    }

    fn set_status(&mut self, message: String) {
        self.status = Some(message);
    }

    fn clear_error(&mut self) {
        self.status = None;
    }
    
    fn init_for_evm(&mut self, context: &mut AppContext) -> Option<String> {
        let strat = match &context.current_strategy {
            Some(s) => s,
            None => {
                self.set_status("No strategy selected".to_string());
                return None;
            }
        };
        
        let quote = match &context.quote {
            Some(q) => q,
            None => {
                self.set_status("Quote not available".to_string());
                return None;
            }
        };
        
        let strategies_map = match &quote.strategies_map {
            Some(map) => map,
            None => {
                self.set_status("Strategies map not available".to_string());
                return None;
            }
        };
        
        let strategy_info = match strategies_map.get(strat) {
            Some(info) => info,
            None => {
                self.set_status(format!("Strategy '{}' not found in map", strat));
                return None;
            }
        };
        
        let redeemer = strategy_info.source_chain_address.clone();
        
        // Get init data
        let init_data = match context.current_order {
            Some(ref order) => order.to_sol_initiate(&redeemer),
            None => {
                self.set_status("Current order not available".to_string());
                return None;
            }
        };
        
        
        let provider_url = match Url::from_str("http://localhost:8546") {
            Ok(url) => url,
            Err(e) => {
                self.set_status(format!("Invalid provider URL: {}", e));
                return None;
            }
        };
        
        let provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(context.eth_wallet.clone())
            .on_http(provider_url);
        
        // Get signature with error handling
        let runtime = match tokio::runtime::Runtime::new() {
            Ok(r) => r,
            Err(e) => {
                self.set_status(format!("Failed to create runtime: {}", e));
                return None;
            }
        };
        
        let sig = runtime.block_on(init_and_get_sig(init_data, provider, context.signer.clone(), &strategy_info.source_asset.asset));
        
        let init_req = InitiateRequest {
            signature: alloy::hex::encode(sig.as_bytes()),
            perform_on: "Source".to_string(),
            order_id: self.order_id.clone(),
        };

        let tx = match context.orderbook.as_ref().unwrap().clone().initiate(init_req) {
            Ok(tx) => {
                Some(tx)
            },
            Err(e) => {
                self.set_status(format!("Initiation failed: {}", e));
                None
            }
        };
        
        tx
    }
}

impl State for OrderDashboardState {
    fn draw(&self, frame: &mut Frame, _context: &mut AppContext) {
        let size = frame.area();
        
        let title_span = vec![Span::raw("Order Dashboard")];
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
                Constraint::Length(3),  // Order ID
                Constraint::Length(3),  // Status
                Constraint::Min(0)      // Instructions
            ].as_ref())
            .split(size);
        
        // Title
        frame.render_widget(
            Paragraph::new(vec![title_line])
                .block(title_block)
                .alignment(Alignment::Center),
            chunks[0],
        );
          
        let output_block = Block::default()
            .title("Order ID")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White));
            
        frame.render_widget(
            Paragraph::new(self.order_id.clone())
                .block(output_block)
                .alignment(Alignment::Left),
            chunks[2],
        );
        
        // Error message box
        let error_style = Style::default().fg(Color::White);
        let error_block = Block::default()
            .title("Status")
            .borders(Borders::ALL)
            .style(error_style);
        
        let status_message = match &self.status {
            Some(msg) => msg.clone(),
            None => "No errors".to_string(),
        };
        
        frame.render_widget(
            Paragraph::new(status_message)
                .block(error_block)
                .alignment(Alignment::Left),
            chunks[3],
        );
        
        let instructions_spans = vec![
            Span::styled("q: Quit | ", Style::default().fg(Color::Red)),
            Span::styled("s: Create Order | ", Style::default().fg(Color::Red)),
            Span::styled("b: Back To Strategy Selection | ", Style::default().fg(Color::Green)),
            Span::styled("c: Clear Error", Style::default().fg(Color::Yellow)),
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
            KeyCode::Char('c') => {
                self.clear_error();
                None
            },
            KeyCode::Char('s') => {
                // Clear any previous errors
                self.clear_error();
                
                // Create order with proper error handling
                let order_result = match &context.orderbook {
                    Some(orderbook) => {
                        match &context.current_order {
                            Some(order) => orderbook.clone().create_order(order.clone()),
                            None => {
                                self.set_status("No current order available".to_string());
                                return None;
                            }
                        }
                    },
                    None => {
                        self.set_status("Orderbook not initialized".to_string());
                        return None;
                    }
                };
                
                // Check if order creation was successful
                match order_result {
                    Ok(order_id) => {
                        self.order_id = order_id.trim_matches('"').to_string();
                        self.set_status("Order Creation Successful".to_string());
                        let timeout_duration = Duration::from_secs(10);
                        sleep(timeout_duration);
                    },
                    Err(e) => {
                        self.set_status(format!("Failed to create order: {}", e));
                    }
                }
                
                if !context.current_strategy.as_ref().unwrap().starts_with("b") {
                    self.init_for_evm(context);
                    self.set_status("Init Successful".to_string());
                } else {
                    let get_matched_order = context.orderbook.as_mut().unwrap().get_matched_order(&self.order_id).unwrap();
                    eprintln!("pay to this htlc addr {:#?} ", get_matched_order.source_swap.swap_id);
                }
                
                let _ = context.orderbook.as_mut().unwrap().wait_for_destination_init(&self.order_id).unwrap();
                let secret_str = hex::encode(context.secret);
                let redeem_tx = context.orderbook.as_mut().unwrap().redeem(&self.order_id, &secret_str).unwrap_or("".to_string());
                if !redeem_tx.is_empty() {
                    self.set_status("Redeem Successful".to_string());
                    return None
                } else {
                    self.set_status("Redeem Unsuccessful".to_string());
                    return None
                }
            },
            KeyCode::Enter => None,
            KeyCode::Backspace => None,
            _ => None,
        }
    }
}