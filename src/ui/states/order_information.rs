use anyhow::anyhow;
use bitcoin::consensus::encode::serialize_hex;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use serde_json::json;
use std::thread::sleep;
use std::time::Duration;

use crate::context::AppContext;
use crate::service::blockchain::bitcoin::htlc::BitcoinHTLC;
use crate::service::blockchain::bitcoin::htlc_handler::HtlcHandler;
use crate::service::blockchain::evm::init_and_get_sig;
use crate::service::blockchain::evm::Initiate;
use crate::service::garden::types::InitiateRequest;

use super::{State, StateType};

pub enum OrderProgress {
    NotStarted,
    OrderCreated,
    Initialized,
    DestinationInitialized,
    Redeemed,
    Failed(String),
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
            .constraints(
                [
                    Constraint::Length(3), // Title
                    Constraint::Length(1),
                    Constraint::Length(3), // Order ID
                    Constraint::Length(3), // Status
                    Constraint::Min(0),    // Instructions
                ]
                .as_ref(),
            )
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
            Span::styled(
                "b: Back To Strategy Selection | ",
                Style::default().fg(Color::Green),
            ),
            Span::styled("c: Clear Error", Style::default().fg(Color::Yellow)),
        ];

        frame.render_widget(
            Paragraph::new(vec![Line::from(instructions_spans)]).alignment(Alignment::Center),
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
            }
            KeyCode::Char('s') => {
                // Clear any previous errors
                self.clear_error();

                match self.progress {
                    OrderProgress::NotStarted => {
                        // Create order
                        let order_result = match &context.order.current_order {
                            Some(order) => context.api.orderbook.create_order(order.clone()),
                            None => {
                                self.set_status("No current order available".to_string());
                                return None;
                            }
                        };

                        // Check if order creation was successful
                        match order_result {
                            Ok(order_id) => {
                                self.order_id = order_id.trim_matches('"').to_string();
                                self.set_status(
                                    "Order Created. Press 's' to initialize".to_string(),
                                );
                                self.progress = OrderProgress::OrderCreated;
                                sleep(Duration::from_secs(5));
                            }
                            Err(e) => {
                                self.set_status(format!("Failed to create order: {}", e));
                                self.progress = OrderProgress::Failed(e.to_string());
                            }
                        }
                    }
                    OrderProgress::OrderCreated => {
                        match context.api.orderbook.get_matched_order(&self.order_id) {
                            Ok(matched_order) => {
                                let swap = matched_order.source_swap;
                                if swap.chain.contains("bitcoin"){
                                    let htlc_handler = HtlcHandler::new(
                                        context.wallet.btc_network,
                                        context.wallet.provider_urls.get("bitcoin").unwrap(),
                                    )
                                    .unwrap();
                                    
    
                                    let secret_hash_bytes = hex::decode(swap.secret_hash).unwrap();
    
                                    let htlc = BitcoinHTLC::new(
                                        secret_hash_bytes,
                                        swap.initiator,
                                        swap.redeemer,
                                        swap.timelock,
                                        context.wallet.btc_network,
                                    )
                                    .unwrap();
                                    let amount: i64 = 9999;
                                    let tx = htlc_handler
                                        .initaite_htlc(
                                            context.wallet.btc_private_key,
                                            htlc.address().expect("failed to get address"),
                                            amount,
                                        )
                                        .unwrap();
                                    let runtime = tokio::runtime::Runtime::new()
                                        .map_err(|e| anyhow!("Unable to create runtime: {}", e))
                                        .unwrap();
    
                                    let txid =
                                        runtime.block_on(htlc_handler.broadcast_tx(&tx)).unwrap();
    
                                    self.set_status(format!(
                                        "Initialized. tx {} Press 's' to wait for destination",
                                        txid
                                    ));
                                    self.progress = OrderProgress::Initialized;
                                }else {
                                    let init_data = Initiate::try_from(&swap).unwrap();
                                    let runtime = tokio::runtime::Runtime::new()
                                        .map_err(|e| anyhow!("Unable to create runtime: {}", e))
                                        .unwrap();
        
                                    let signature = runtime
                                        .block_on(init_and_get_sig(init_data, &context.wallet.provider_urls["ethereum"], context.wallet.signer.clone(), &swap.asset));

                                    let init_req = InitiateRequest{
                                        order_id: self.order_id.to_string(),
                                        signature: signature.to_string(),
                                        perform_on: "Source".to_string()
                                    };
                                    
                                    let tx = context.api.orderbook.initiate(init_req).unwrap();
                                    self.set_status(format!(
                                        "Initialized. tx {} Press 's' to wait for destination",
                                        tx
                                    ));
                                    self.progress = OrderProgress::Initialized;
                                }
                                
                            }
                            Err(e) => {
                                self.set_status(format!("Failed to get matched order: {}", e));
                                self.progress = OrderProgress::Failed(e.to_string());
                            }
                        }
                    }
                    OrderProgress::Initialized => {
                        // Wait for destination init
                        self.set_status("Waiting for destination init...".to_string());
                        match context
                            .api
                            .orderbook
                            .wait_for_destination_init(&self.order_id)
                        {
                            Ok(_) => {
                                self.set_status(
                                    "Destination initialized. Press 's' to redeem".to_string(),
                                );
                                self.progress = OrderProgress::DestinationInitialized;
                            }
                            Err(e) => {
                                self.set_status(format!("Failed waiting for destination: {}", e));
                                self.progress = OrderProgress::Failed(e.to_string());
                            }
                        }
                    }
                    OrderProgress::DestinationInitialized => {
                        let secret_str = hex::encode(context.order.secret);
                        if context
                            .order
                            .current_order
                            .as_ref()
                            .unwrap()
                            .destination_chain
                            .contains("bitcoin")
                        {
                            let swap = context
                                .api
                                .orderbook
                                .get_matched_order(&self.order_id)
                                .unwrap()
                                .destination_swap;

                            let secret_hash_bytes = hex::decode(swap.secret_hash).unwrap();

                            let htlc = BitcoinHTLC::new(
                                secret_hash_bytes,
                                swap.initiator,
                                swap.redeemer,
                                swap.timelock,
                                context.wallet.btc_network,
                            )
                            .unwrap();
                            let witness_stack =
                                htlc.redeem(&context.order.secret.to_vec()).unwrap();
                            let htlc_handler = HtlcHandler::new(
                                context.wallet.btc_network,
                                context.wallet.provider_urls.get("bitcoin").unwrap(),
                            )
                            .unwrap();

                            let runtime = tokio::runtime::Runtime::new()
                                .map_err(|e| anyhow!("Unable to create runtime: {}", e))
                                .unwrap();

                            let tx = runtime
                                .block_on(htlc_handler.create_redeem_tx(
                                    htlc.address().unwrap(),
                                    witness_stack,
                                    Some("".to_string()),
                                    context.wallet.btc_private_key,
                                    0,
                                ))
                                .unwrap();

                            let tx_hex = serialize_hex(&tx);

                            match context.api.orderbook.btc_redeem(&self.order_id, &tx_hex) {
                                Ok(tx) if !tx.is_empty() => {
                                    self.set_status(format!("Redeem Successful!! {} ", tx));
                                    self.progress = OrderProgress::Redeemed;
                                }
                                Ok(_) => {
                                    self.set_status(
                                        "Redeem returned empty transaction".to_string(),
                                    );
                                    self.progress =
                                        OrderProgress::Failed("Empty transaction".to_string());
                                }
                                Err(e) => {
                                    self.set_status(format!("Redeem failed: {}", e));
                                    self.progress = OrderProgress::Failed(e.to_string());
                                }
                            }
                        } else {
                            match context.api.orderbook.redeem(&self.order_id, &secret_str) {
                                Ok(tx) if !tx.is_empty() => {
                                    self.set_status(format!("Redeem Successful!! {} ", tx));
                                    self.progress = OrderProgress::Redeemed;
                                }
                                Ok(_) => {
                                    self.set_status(
                                        "Redeem returned empty transaction".to_string(),
                                    );
                                    self.progress =
                                        OrderProgress::Failed("Empty transaction".to_string());
                                }
                                Err(e) => {
                                    self.set_status(format!("Redeem failed: {}", e));
                                    self.progress = OrderProgress::Failed(e.to_string());
                                }
                            }
                        }
                    }
                    OrderProgress::Redeemed => {
                        self.set_status(
                            "Order process complete! Press 'c' to start over.".to_string(),
                        );
                    }
                    OrderProgress::Failed(ref reason) => {
                        self.set_status(format!("Process failed: {}. Press 'c' to retry.", reason));
                    }
                }
                None
            }
            KeyCode::Enter => None,
            KeyCode::Backspace => None,
            _ => None,
        }
    }
}
