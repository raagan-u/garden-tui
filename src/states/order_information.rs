use std::str::FromStr;

use alloy::providers::ProviderBuilder;
use crossterm::event::{KeyEvent, KeyCode};
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use ratatui::prelude::*;
use reqwest::Url;

use crate::app::AppContext;


use super::{State, StateType};

pub struct OrderDashboardState {
    pub order_id: String,
}

impl OrderDashboardState {
    pub fn new() -> Self {
        OrderDashboardState {
            order_id: "Press 's' to create-order".to_string()
        }
    }
}
impl State for OrderDashboardState {
    fn draw(&self, frame: &mut Frame, _context: &mut AppContext){
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
          
        let output_block = Block::default()
            .title("Order ID")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White));
            
        // Fix: Don't use render_stateful_widget for a string
        frame.render_widget(
            Paragraph::new(self.order_id.clone())
                .block(output_block)
                .alignment(Alignment::Left),
            chunks[2],
        );
        
        let instructions_spans = vec![
            Span::styled("q: Quit | ", Style::default().fg(Color::Red)),
            Span::styled("s: Create Order | ", Style::default().fg(Color::Red)),
            Span::styled("b: Back To Strategy Selection | ", Style::default().fg(Color::Green)),
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
            KeyCode::Char('s') => {
                    let ob = context.orderbook.as_ref().unwrap().clone().create_order(context.current_order.as_ref().unwrap().clone());
                    self.order_id = format!("ordered creation successful. order id {}", ob.unwrap());
                    let strat = context.current_strategy.as_ref().unwrap();
                    let redeemer = context.quote.as_ref().unwrap().strategies_map.as_ref().unwrap().get(strat).unwrap().dest_chain_address.clone();
                    let init_data = context.current_order.as_ref().unwrap().to_sol_initiate(&redeemer);
                    let provider = ProviderBuilder::new()
                        .with_recommended_fillers()
                        .wallet(context.eth_wallet.clone())
                        .on_http(Url::from_str("http://localhost:8546").unwrap());
                    
                None
            },
            KeyCode::Enter => None,
            KeyCode::Backspace => None,
            _ => None,
        }
    }
}