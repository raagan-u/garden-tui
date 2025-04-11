// src/app.rs
use std::{fs::File, io::Read, str::FromStr};
use alloy::signers::{k256::ecdsa::SigningKey, local::{LocalSigner, PrivateKeySigner}};
use bitcoin::{key::Secp256k1, PublicKey};
use ratatui::{
    Frame, 
    widgets::ListState
};
use crossterm::event::KeyEvent;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{garden_api::{orderbook::Orderbook, quote::Quote, types::Order}, states::{
    network_information::NetworkInformationState, network_selection::NetworkSelectionState, order_information::OrderDashboardState, strategy_selector::StrategySelector, swap_information::SwapDashboardState, State, StateType
}};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NetworkUrls {
    pub evm_relayer_url: String,
    pub quote_server_url: String,
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
    pub secret: [u8; 32],
    pub signer: LocalSigner<SigningKey>,
    pub _btc_pubkey: bitcoin::XOnlyPublicKey
}

pub struct App {
    pub context: AppContext,
    state: Box<dyn State>,
    pub should_quit: bool,
}

impl App {
    pub fn new(priv_key: &str) -> App {
        let mut network_list_state = ListState::default();
        network_list_state.select(Some(0));
        
        let signer = PrivateKeySigner::from_str(priv_key).unwrap();
        let priv_key_bytes = hex::decode(priv_key).unwrap();
        let sk = bitcoin::PrivateKey::from_slice(&priv_key_bytes, bitcoin::Network::Regtest).unwrap();
        let secp = Secp256k1::new();
        
        let (btc_pubkey, _) = PublicKey::from_private_key(&secp, &sk).inner.x_only_public_key();
        
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
            signer: signer.clone(),
            secret: [0; 32],
            _btc_pubkey: btc_pubkey
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
                StateType::SwapInformation => {
                  self.state = Box::new(SwapDashboardState::new());  
                },
                StateType::OrderInformation => {
                  self.state = Box::new(OrderDashboardState::new());  
                },
                StateType::Quit => {
                    self.should_quit = true;
                }
            }
        }
    }
    
    pub fn get_final_message(&self) -> Option<String> {
        self.context.final_message.clone()
    }
}