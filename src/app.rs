use std::{fs::File, io::Read, str::FromStr};
use alloy::signers::local::PrivateKeySigner;
use bitcoin::{key::Secp256k1, PublicKey};
use ratatui::{
    Frame, 
    widgets::ListState
};
use crossterm::event::KeyEvent;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::ui::states::{
    network_information::NetworkInformationState, order_information::OrderDashboardState, swap_information::SwapDashboardState, State, StateType
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NetworkUrls {
    pub evm_relayer_url: String,
    pub quote_server_url: String,
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
            selected_network_urls: None,
            selected_network: None,
            exit_message: None
        };
        
        App {
            context,
            state: Box::new(NetworkInformationState::new()),
            should_quit: false,
        }
    }
    
    pub fn load_config(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut file = File::open("api.json")?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        let config = serde_json::from_str::<Value>(&contents)?;
        self.context.api_urls = Some(config["api_urls"].clone());
        self.context.provider_urls = Some(config["provider_urls"].clone());
        Ok(())
    }
    
    pub fn draw(&mut self, frame: &mut Frame) {
        self.state.draw(frame, &mut self.context);
    }
    
    pub fn handle_key(&mut self, key: KeyEvent) {
        let next_state = self.state.handle_key(key, &mut self.context);
        
        if let Some(state_type) = next_state {
            match state_type {
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
        self.context.exit_message.clone()
    }
}