// src/app.rs
use std::{fs::File, io::Read, str::FromStr};
use alloy::{network::EthereumWallet, signers::{k256::ecdsa::SigningKey, local::{LocalSigner, PrivateKeySigner}}};
use ratatui::{
    Frame, 
    widgets::ListState
};
use crossterm::event::KeyEvent;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{garden_api::{orderbook::Orderbook, quote::Quote, types::Order}, states::{
    network_information::NetworkInformationState, network_selection::NetworkSelectionState, strategy_selector::StrategySelector, swap_information::SwapDashboardState, State, StateType
}};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NetworkUrls {
    pub evm_relayer_url: String,
    pub quote_server_url: String,
    pub virtual_balance_server_url: String,
}

pub struct AppContext {
    pub network_list_state: ListState,
    pub networks: Vec<&'static str>,
    pub api_urls: Option<Value>,
    pub selected_network_urls: Option<NetworkUrls>,
    pub selected_network: Option<String>,
    pub final_message: Option<String>,
    pub quote: Option<Quote>,
    pub orderbook: Option<Orderbook>,
    pub current_strategy: Option<String>,
    pub strategy_selector: Option<StrategySelector>,
    pub current_order: Option<Order>,
    pub eth_wallet: EthereumWallet,
    pub signer: LocalSigner<SigningKey>
}

pub struct App {
    pub context: AppContext,
    state: Box<dyn State>,
    pub should_quit: bool,
}

impl App {
    pub fn new(eth_priv_key: &str) -> App {
        let mut network_list_state = ListState::default();
        network_list_state.select(Some(0));
        
        let signer = PrivateKeySigner::from_str(eth_priv_key).unwrap();
        let context = AppContext {
            network_list_state,
            networks: vec!["Mainnet", "Testnet", "Localnet"],
            api_urls: None,
            selected_network_urls: None,
            selected_network: None,
            final_message: None,
            quote: None,
            orderbook: None,
            current_strategy: None,
            strategy_selector: None,
            current_order: None,
            eth_wallet: EthereumWallet::from(signer.clone()),
            signer: signer.clone()
        };
        
        App {
            context,
            state: Box::new(NetworkSelectionState::new()),
            should_quit: false,
        }
    }
    
    pub fn load_api_urls(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut file = File::open("api.json")?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        
        self.context.api_urls = Some(serde_json::from_str(&contents)?);
        Ok(())
    }
    
    pub fn draw(&mut self, frame: &mut Frame) {
        self.state.draw(frame, &mut self.context);
    }
    
    pub fn handle_key(&mut self, key: KeyEvent) {
        // Get the next state type from the current state
        let next_state = self.state.handle_key(key, &mut self.context);
        
        // Handle state transitions
        if let Some(state_type) = next_state {
            match state_type {
                StateType::NetworkSelection => {
                    self.state = Box::new(NetworkSelectionState::new());
                },
                StateType::NetworkInformation => {
                    self.state = Box::new(NetworkInformationState::new());
                },
                StateType::Swapinformation => {
                  self.state = Box::new(SwapDashboardState::new());  
                },
                StateType::Quit => {
                    self.should_quit = true;
                },
                StateType::Exit(message) => {
                    self.context.final_message = Some(message);
                    self.should_quit = true;
                },
            }
        }
    }
    
    pub fn get_final_message(&self) -> Option<String> {
        self.context.final_message.clone()
    }
}