use std::{fs::File, io::Read};

use ratatui::widgets::ListState;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NetworkUrls {
    pub evm_relayer_url: String,
    pub quote_server_url: String,
    pub virtual_balance_server_url: String,
}

pub enum AppState {
    NetworkSelection,
    NetworkInformation,
}

pub struct App {
    pub state: AppState,
    pub network_list_state: ListState,
    pub networks: Vec<&'static str>,
    pub api_urls: Option<Value>,
    pub selected_network_urls: Option<NetworkUrls>,
    pub selected_network: Option<String>,
}

impl App {
    pub fn new() -> App {
        let mut network_list_state = ListState::default();
        network_list_state.select(Some(0));
        
        App {
            state: AppState::NetworkSelection,
            network_list_state,
            networks: vec!["Mainnet", "Testnet", "Localnet"],
            api_urls: None,
            selected_network_urls: None,
            selected_network: None,
        }
    }

    pub fn next(&mut self) {
        let i = match self.network_list_state.selected() {
            Some(i) => {
                if i >= self.networks.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.network_list_state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.network_list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.networks.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.network_list_state.select(Some(i));
    }

    pub fn load_api_urls(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut file = File::open("api.json")?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        
        self.api_urls = Some(serde_json::from_str(&contents)?);
        Ok(())
    }

    pub fn select_network(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(api_urls) = &self.api_urls {
            if let Some(selected) = self.network_list_state.selected() {
                let network_key = self.networks[selected].to_lowercase();
                self.selected_network = Some(self.networks[selected].to_string());
                
                if let Some(network_config) = api_urls.get(&network_key) {
                    // Parse the network config object
                    if let Some(evm_url) = network_config.get("evm_relayer_url").and_then(|v| v.as_str()) {
                        if let Some(quote_url) = network_config.get("quote_server_url").and_then(|v| v.as_str()) {
                            if let Some(vb_url) = network_config.get("virtual_balance_server_url").and_then(|v| v.as_str()) {
                                self.selected_network_urls = Some(NetworkUrls {
                                    evm_relayer_url: evm_url.to_string(),
                                    quote_server_url: quote_url.to_string(),
                                    virtual_balance_server_url: vb_url.to_string(),
                                });
                                
                                // Change app state to display network info
                                self.state = AppState::NetworkInformation;
                                return Ok(());
                            }
                        }
                    }
                }
                
                return Err(format!("Could not find valid URLs for network: {}", network_key).into());
            }
        }
        
        Err("API URLs not loaded".into())
    }
    
    pub fn back_to_selection(&mut self) {
        self.state = AppState::NetworkSelection;
    }
}