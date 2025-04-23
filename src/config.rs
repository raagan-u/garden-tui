use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub networks: HashMap<String, NetworkConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub api: ApiConfig,
    pub providers: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    pub quote_server_url: String,
    pub authenticator_url: String,
    pub evm_relayer_url: String,
    pub orderbook_url: String,
}

impl Config {
    pub fn from_json(json_str: &str) -> Result<Self> {
        serde_json::from_str(json_str)
            .context("Failed to parse JSON string into Config")
    }

    
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_str = path.as_ref().display().to_string();
        let mut file = File::open(&path)
            .with_context(|| format!("Failed to open config file at {}", path_str))?;
        
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .with_context(|| format!("Failed to read contents from config file at {}", path_str))?;
        
        Self::from_json(&contents)
            .with_context(|| format!("Failed to parse config file at {}", path_str))
    }
    
    
    pub fn get_network(&self, network_name: &str) -> Result<&NetworkConfig> {
        self.networks.get(network_name)
            .with_context(|| format!("Network '{}' not found in configuration", network_name))
    }
}
