use std::env;
use std::thread::sleep;
use std::time::Duration;
use bitcoin::consensus::encode::serialize_hex;
use crossterm::event::{KeyEvent, KeyCode};
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use ratatui::prelude::*;


use crate::app::AppContext;
use crate::garden_api::types::InitiateRequest;
use crate::htlc::bitcoin_htlc::BitcoinHTLC;
use crate::htlc::utils::create_tx;
use crate::htlc::utils::init_and_get_sig;
use crate::htlc::utils::pay_to_htlc;

use super::{State, StateType};

pub enum OrderProgress {
    NotStarted,
    OrderCreated,
    Initialized,
    DestinationInitialized,
    Redeemed,
    Failed(String)
}

pub struct OrderDashboardState {
    pub order_id: String,
    pub status: Option<String>,
     pub progress: OrderProgress,
}

impl OrderDashboardState {
    pub fn new() -> Self {
        OrderDashboardState {
            order_id: "Press 's' to create-order".to_string(),
            status: None,
            progress: OrderProgress::NotStarted,
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
         
        let strategy_info = match quote.strategies_map.get(strat) {
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
        
        let rpc_url = context.provider_urls.as_ref().unwrap()["localnet"][&strategy_info.source_chain].to_string().trim_matches('"').to_string();
        // Get signature with error handling
        let runtime = match tokio::runtime::Runtime::new() {
            Ok(r) => r,
            Err(e) => {
                self.set_status(format!("Failed to create runtime: {}", e));
                return None;
            }
        };
        
        let sig = runtime.block_on(init_and_get_sig(init_data, &rpc_url, context.signer.clone(), &strategy_info.source_asset.asset));
        
        let init_req = InitiateRequest {
            signature: alloy::hex::encode(sig.as_bytes()),
            perform_on: "Source".to_string(),
            order_id: self.order_id.clone(),
        };

        let tx = match context.orderbook.as_ref().unwrap().initiate(init_req) {
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
    
    fn init_for_btc(&self, context: &mut AppContext) -> Option<String> {
        let swap = context.orderbook.as_mut().unwrap().get_matched_order(&self.order_id).unwrap().source_swap;
        let secret_hash = hex::decode(swap.secret_hash).unwrap();
        let network = match &context.selected_network{
            Some(current_network) => {
                match current_network.as_str() {
                    "mainnet" => bitcoin::Network::Bitcoin,
                    "testnet" => bitcoin::Network::Testnet4,
                    _ => bitcoin::Network::Regtest
                }
            },
            None => bitcoin::Network::Regtest
        };
        let htlc = BitcoinHTLC::new(secret_hash, swap.initiator, swap.redeemer, swap.timelock as i64, network).unwrap();
        let priv_key_hex = env::var("PRIV_KEY").unwrap();
        let indexer_url = context.provider_urls.as_ref().unwrap()["localnet"]["bitcoin"].to_string().trim_matches('"').to_string();
        let tx = pay_to_htlc(&priv_key_hex, htlc.address().unwrap(), swap.amount.to_string().parse::<i64>().unwrap(), &indexer_url, network).unwrap();
        Some(tx)
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
                
                match self.progress {
                    OrderProgress::NotStarted => {
                        // Create order
                        let order_result = match &context.orderbook {
                            Some(orderbook) => {
                                match &context.current_order {
                                    Some(order) => orderbook.create_order(order.clone()),
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
                                self.set_status("Order Created. Press 's' to initialize".to_string());
                                self.progress = OrderProgress::OrderCreated;
                                sleep(Duration::from_secs(5));
                            },
                            Err(e) => {
                                self.set_status(format!("Failed to create order: {}", e));
                                self.progress = OrderProgress::Failed(e.to_string());
                            }
                        }
                    },
                    OrderProgress::OrderCreated => {
                        // Initialize
                        if !context.current_strategy.as_ref().unwrap().starts_with("b") {
                            match self.init_for_evm(context) {
                                Some(tx) => {
                                    self.set_status(format!("Initialized. tx {} Press 's' to wait for destination", tx));
                                    self.progress = OrderProgress::Initialized;
                                },
                                None => {
                                    // Error already set in init_for_evm
                                    self.progress = OrderProgress::Failed("Initialization failed".to_string());
                                }
                            }
                        } else {
                            match context.orderbook.as_mut().unwrap().get_matched_order(&self.order_id) {
                                Ok(_) => {
                                    let tx = self.init_for_btc(context).unwrap();
                                    self.set_status(format!("Initialized. tx {} Press 's' to wait for destination", tx));
                                    self.progress = OrderProgress::Initialized;
                                },
                                Err(e) => {
                                    self.set_status(format!("Failed to get matched order: {}", e));
                                    self.progress = OrderProgress::Failed(e.to_string());
                                }
                            }
                        }
                    },
                    OrderProgress::Initialized => {
                        // Wait for destination init
                        self.set_status("Waiting for destination init...".to_string());
                        match context.orderbook.as_mut().unwrap().wait_for_destination_init(&self.order_id) {
                            Ok(_) => {
                                self.set_status("Destination initialized. Press 's' to redeem".to_string());
                                self.progress = OrderProgress::DestinationInitialized;
                            },
                            Err(e) => {
                                self.set_status(format!("Failed waiting for destination: {}", e));
                                self.progress = OrderProgress::Failed(e.to_string());
                            }
                        }
                    },
                    OrderProgress::DestinationInitialized => {
                        // Redeem
                        let secret_str = hex::encode(context.secret);
                        if context.current_order.as_ref().unwrap().destination_chain.contains("bitcoin") {
                            let matched_order = context.orderbook.as_mut().unwrap().get_matched_order(&self.order_id).unwrap();
                            let swap = matched_order.destination_swap;
                            let secret_hash = hex::decode(swap.secret_hash).unwrap();
                            let network = match &context.selected_network{
                                Some(current_network) => {
                                    match current_network.as_str() {
                                        "mainnet" => bitcoin::Network::Bitcoin,
                                        "testnet" => bitcoin::Network::Testnet4,
                                        _ => bitcoin::Network::Regtest
                                    }
                                },
                                None => bitcoin::Network::Regtest
                            };
                            let htlc = BitcoinHTLC::new(secret_hash, swap.initiator, swap.redeemer, swap.timelock as i64, network).unwrap();
                            let witness_stack = htlc.redeem(&context.secret.to_vec()).unwrap();
                            let priv_key = env::var("PRIV_KEY").unwrap();
                            let runtime = match tokio::runtime::Runtime::new() {
                                Ok(r) => r,
                                Err(e) => {
                                    self.set_status(format!("Failed to create runtime: {}", e));
                                    return None;
                                }
                            };
                            
                            let btc_recipient = matched_order.create_order.additional_data.bitcoin_optional_recipient;
                            let tx = runtime.block_on(create_tx(htlc.address().unwrap(), witness_stack, btc_recipient, &priv_key, network)).unwrap();
                            let tx_hex = serialize_hex(&tx);
                            match context.orderbook.as_mut().unwrap().btc_redeem(&self.order_id, &tx_hex) {
                                Ok(tx) if !tx.is_empty() => {
                                    self.set_status(format!("Redeem Successful!! {} ", tx));
                                    self.progress = OrderProgress::Redeemed;
                                },
                                Ok(_) => {
                                    self.set_status("Redeem returned empty transaction".to_string());
                                    self.progress = OrderProgress::Failed("Empty transaction".to_string());
                                },
                                Err(e) => {
                                    self.set_status(format!("Redeem failed: {}", e));
                                    self.progress = OrderProgress::Failed(e.to_string());
                                }
                            }
                            
                            
                        } else {
                            match context.orderbook.as_mut().unwrap().redeem(&self.order_id, &secret_str) {
                                Ok(tx) if !tx.is_empty() => {
                                    self.set_status(format!("Redeem Successful!! {} ", tx));
                                    self.progress = OrderProgress::Redeemed;
                                },
                                Ok(_) => {
                                    self.set_status("Redeem returned empty transaction".to_string());
                                    self.progress = OrderProgress::Failed("Empty transaction".to_string());
                                },
                                Err(e) => {
                                    self.set_status(format!("Redeem failed: {}", e));
                                    self.progress = OrderProgress::Failed(e.to_string());
                                }
                            }
                        }
                    },
                    OrderProgress::Redeemed => {
                        self.set_status("Order process complete! Press 'c' to start over.".to_string());
                    },
                    OrderProgress::Failed(ref reason) => {
                        self.set_status(format!("Process failed: {}. Press 'c' to retry.", reason));
                    }
                }
                None
            },
            KeyCode::Enter => None,
            KeyCode::Backspace => None,
            _ => None,
        }
    }
}