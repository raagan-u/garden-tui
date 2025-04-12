use std::collections::HashMap;

use anyhow::{anyhow, bail, Context, Result};
use bitcoin::hex::DisplayHex;
use rand::TryRngCore;
use serde_json::Value;
use sha2::{Digest, Sha256};
use super::types::{Order, Strategy};

#[derive(Debug, Clone)]
pub struct Quote {
    client: reqwest::blocking::Client,
    url: String,
    pub strategies_map: HashMap<String, Strategy>,
}

impl Quote {
    
    pub fn new(client: reqwest::blocking::Client, url: String) -> Result<Self> {
        let resp = client.get(format!("{}/quote/strategies", url)).send().unwrap();
        
        let response = resp.json::<Value>().map_err(|e| anyhow!("error fetching strategies from quote {}", e)).unwrap();
        
        let strategies: HashMap<String, Strategy> = serde_json::from_value(response["result"].clone())
            .map_err(|e| anyhow!("error deserializing strategies: {}", e)).unwrap();
        
        if !strategies.is_empty() {
            Ok(Self { client, url , strategies_map: strategies })
        } else {
            bail!("no strategies found");
        }
        
    }
    
    pub  fn get_price(&self, order_pair: &str, amount: &str) -> Result<String> {
        let url = format!("{}/quote?order_pair={}&amount={}&exact_out={}", self.url, order_pair, amount, false);
        
        let resp = self.client.get(&url).send()?;
        
        let response = resp.json::<Value>().map_err(|e| anyhow!("error getting quote price {}", e))?;
        let output_amount = response["result"]["quotes"].clone();
        let final_price = output_amount
            .as_object()
            .ok_or_else(|| anyhow!("output amount is not an object"))?
            .values()
            .next()
            .ok_or_else(|| anyhow!("output amount object is empty"))?
            .to_string();
        
        Ok(final_price.trim_matches('"').to_string())
    }
    
    pub  fn get_attested_quote(&self, order: Order) -> Result<Order> {
        let url = format!("{}/quote/attested", self.url);
        let resp = self.client.post(&url).json(&order).send()?;
        
        let mut response = resp.json::<Value>()
            .context("error getting attested quote")?;
        
        let Some(result) = response.get_mut("result") else {
            bail!("missing result field in response {response}");
        };
        
        let attested_order: Order = serde_json::from_value(result.take())
            .context(format!("error parsing order from attested quote "))?;
        
        Ok(attested_order)
    }
    
    pub fn strategy_readable(&self, strategy_id: &str) -> Result<String> {
        let strategy = self.strategies_map.get(strategy_id).expect("failed to retrieve strategy");
        let readable_strat = format!("{} to {}", strategy.source_chain, strategy.dest_chain);
        Ok(readable_strat)
    }
}

pub fn generate_secret() -> Result<([u8; 32], [u8; 32])> {
    let mut secret = [0u8; 32];

    rand::rng().try_fill_bytes(&mut secret).unwrap();
    let mut hasher = Sha256::new();
    hasher.update(secret);
    let hash = hasher.finalize();

    let hash_bytes = hex::decode(hash.to_lower_hex_string()).unwrap();
    let mut hash_array = [0u8; 32];
    hash_array.copy_from_slice(&hash_bytes);

    Ok((secret, hash_array))
}