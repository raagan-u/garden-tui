use alloy::signers::{k256::ecdsa::SigningKey, local::LocalSigner};
use ratatui::widgets::ListState;
use serde_json::Value;

use crate::{app::NetworkUrls, service::garden::{orderbook::Orderbook, quote::Quote, types::Order}};

pub struct AppContext {
    pub selected_network_urls: NetworkUrls,
    pub selected_network: String,
    pub exit_message: Option<String>,
    
    pub wallet: WalletContext,
    pub config: ConfigContext,
    pub api: APIContext,
    pub order: OrderContext,
}

impl AppContext {
    pub fn new(network_list_state: ListState, network: Vec<&'static str>,) -> Self {
        Self {
            
        }
    }
}

pub struct WalletContext {
    pub signer: LocalSigner<SigningKey>,
    pub btc_network: bitcoin::Network,
    pub btc_pubkey: bitcoin::XOnlyPublicKey,
    pub btc_address: bitcoin::Address,
}

pub struct APIContext {
    pub quote: Quote,
    pub orderbook: Orderbook,
}

pub struct OrderContext {
    pub current_strategy: Option<String>,
    pub current_order: Option<Order>,
    pub secret: [u8; 32],
}

pub struct ConfigContext {
    pub api_urls: Value,
    pub provider_urls: Value,
}