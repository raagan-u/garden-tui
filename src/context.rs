use std::{collections::HashMap, env, str::FromStr, sync::Arc, time::Duration};

use alloy::signers::{k256::ecdsa::SigningKey, local::{LocalSigner, PrivateKeySigner}};
use bitcoin::{key::Secp256k1, Address, CompressedPublicKey, PrivateKey, PublicKey};
use reqwest::cookie::Jar;


use crate::{config::{ApiConfig, NetworkConfig}, service::garden::{orderbook::Orderbook, quote::Quote, types::Order}};

#[derive(Clone)]
pub struct AppContext {
    pub selected_network: String,
    pub exit_message: Option<String>,
    
    pub wallet: WalletContext,
    pub api: APIContext,
    pub order: OrderContext,
}

impl AppContext {
    pub fn new(selected_network: &str, config: &NetworkConfig) -> Self {
        
        let wallet = WalletContext::new(selected_network, config.providers.clone());
        let api = APIContext::new(config.api.clone(), &wallet.signer);
        let order = OrderContext::default();
        
        Self {
            selected_network: selected_network.to_string(),
            exit_message: None,
            wallet,
            api,
            order
        }
    }
}

#[derive(Clone)]
pub struct WalletContext {
    pub signer: LocalSigner<SigningKey>,
    pub btc_network: bitcoin::Network,
    pub btc_private_key: bitcoin::PrivateKey,
    pub btc_xpubkey: String,
    pub btc_address: String,
    pub provider_urls: HashMap<String, String>
}

impl WalletContext {
    fn new(network: &str, provider_urls: HashMap<String, String>) -> WalletContext {
        let eth_priv_key = env::var("PRIV_KEY").expect("please provide a valid PRIV_KEY in env");
        let btc_priv_key = env::var("BTC_PRIV_KEY").unwrap_or(eth_priv_key.clone());
        
        let signer = PrivateKeySigner::from_str(&eth_priv_key).expect("ERR CREATING ETH SIGNER");
        let btc_network = match network {
            "mainnet" => bitcoin::Network::Bitcoin,
            "testnet" => bitcoin::Network::Testnet4,
            _ => bitcoin::Network::Regtest  
        };
        
        let priv_key_bytes = hex::decode(&btc_priv_key).unwrap();
        let btc_private_key = PrivateKey::from_slice(&priv_key_bytes, btc_network).unwrap();
        let secp = Secp256k1::new();
        let pubkey = PublicKey::from_private_key(&secp, &btc_private_key);
        let btc_pubkey = CompressedPublicKey::try_from(pubkey).unwrap();
        let btc_address = Address::p2wpkh(&btc_pubkey, btc_network).to_string();
        
        Self {
            signer,
            btc_network,
            btc_private_key,
            btc_address,
            btc_xpubkey: btc_pubkey.to_string()[2..].to_string(),
            provider_urls
        }
    }
}

#[derive(Clone)]
pub struct APIContext {
    pub quote: Quote,
    pub orderbook: Orderbook,
}

impl APIContext {
    fn new(api_urls: ApiConfig, signer: &PrivateKeySigner) -> Self {
        let cookie_store = Arc::new(Jar::default());
        let client = reqwest::blocking::ClientBuilder::new()
            .timeout(Duration::from_secs(5))
            .cookie_provider(cookie_store.clone())
            .build()
            .unwrap();
        
        let quote = Quote::new(client.clone(), api_urls.quote_server_url).unwrap();
        let orderbook = Orderbook::new(client, &api_urls.evm_relayer_url, signer);
        
        Self {
            quote,
            orderbook
        }
    }
}

#[derive(Clone)]
pub struct OrderContext {
    pub current_strategy: Option<String>,
    pub current_order: Option<Order>,
    pub secret: [u8; 32],
}

impl Default for OrderContext {
    fn default() -> Self {
        Self {
            current_strategy: None,
            current_order: None,
            secret: [0; 32],
        }
    }
}