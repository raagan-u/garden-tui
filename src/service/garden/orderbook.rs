use std::{thread::sleep, time::Duration};

use alloy::{hex::ToHexExt, signers::{local::PrivateKeySigner, SignerSync}};
use anyhow::{anyhow, Result};
use chrono::Utc;
use serde_json::Value;


use super::types::{InitiateRequest, MatchedOrder, Order};

#[derive(Clone)]
pub struct Orderbook {
    client: reqwest::blocking::Client,
    url: String,
    jwt: String,
}

impl Orderbook {
    pub fn new(client: reqwest::blocking::Client, url: &str, signer: &PrivateKeySigner) -> Self {
        let jwt = authenticate(signer, &url, client.clone()).unwrap();
        Self { client , url: url.to_string(), jwt }
    }

    pub fn create_order(&self, order: Order) -> Result<String> {
        let url = format!("{}/relayer/create-order", self.url);
        let resp = self.client
            .post(url)
            .bearer_auth(&self.jwt)
            .json(&order)
            .send()
            .map_err(|e| anyhow!("failed to send create order request: {}", e))?;
        
        if resp.status() == 401 {
            return Err(anyhow!("401"));
        }
        
        let result = resp.json::<Value>()
            .map_err(|e| anyhow!("failed to parse response as JSON: {}", e))?;
        
        let create_id = result.get("result")
            .ok_or_else(|| anyhow!("missing result field in response: {}", serde_json::to_string_pretty(&result).unwrap()))?
            .clone()
            .to_string();
        Ok(create_id.trim_matches('"').to_string())
    }
    pub fn initiate(&self, init_req: InitiateRequest) -> Result<String> {
        let url = format!("{}/relayer/initiate", self.url);
        let resp = self.client.post(url)
            .bearer_auth(&self.jwt)
            .json(&init_req)
            .send()?;
        let result = resp.json::<Value>()?;
        Ok(result["result"].to_string())
    }
    pub fn wait_for_destination_init(&self, order_id: &str) -> Result<String> {
        let start_time = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(60);
        
        loop {
            if start_time.elapsed() > timeout {
                return Err(anyhow!("Timeout waiting for destination init"));
            }
            
            let matched_order = self.get_matched_order(order_id)
                .map_err(|e| anyhow!("Failed to get matched order: {}", e))?;
                
            if let Some(init_tx_hash) = matched_order.destination_swap.initiate_tx_hash {
                if !init_tx_hash.is_empty(){
                    return Ok(init_tx_hash);
                }
            }
            sleep(Duration::from_secs(5));
        }
    }
    
    pub fn redeem(&self, order_id: &str, secret: &str) -> Result<String> {
        let url = format!("{}/relayer/redeem", self.url);
        let resp = self.client.post(url).bearer_auth(&self.jwt)
            .json(&serde_json::json!({
                "order_id": order_id,
                "secret": secret,
                "perform_on": "Destination"
            }))
            .send()?;
        let result = resp.json::<Value>()?;
        Ok(result["result"].to_string())
    }

    pub fn btc_redeem(&self, order_id: &str, tx_hex: &str) -> Result<String> {
        let url = format!("{}/gasless/order/bitcoin/redeem", self.url);
        let resp = self.client.post(url).bearer_auth(&self.jwt)
            .json(&serde_json::json!({
                "order_id": order_id,
                "redeem_tx_bytes": tx_hex
            }))
            .send()?;
        let result = resp.json::<Value>()?;
        Ok(result["result"].to_string())
    }
    
    pub fn get_matched_order(&self, order_id: &str) -> Result<MatchedOrder> {
        let url = format!("{}/orders/id/matched/{}", self.url, order_id);
        let resp = self.client
            .get(url)
            .send()?;

        let response = resp.json::<Value>()
            .map_err(|e| anyhow!("error getting matched order {}", e))?;

        let result = response.get("result")
            .ok_or_else(|| anyhow!("missing result field in response"))?;
        
        if result.is_null(){
            sleep(Duration::from_secs(5));
            
            return self.get_matched_order(&order_id);
        }
        
        let order: MatchedOrder = serde_json::from_value(result.clone())
            .map_err(|e| anyhow!("failed to parse order: {} and result is {} ", e, response))?;

        Ok(order)
    }
}

fn authenticate(signer: &PrivateKeySigner, url: &str, client: reqwest::blocking::Client) -> Result<String, Box<dyn std::error::Error>> {
    let res = client.post(format!("{}/auth/siwe/challenges", url)).send().expect("error getting nonce");

    let body = res.text()?;
    let response: serde_json::Value = serde_json::from_str(&body)?;
    
    let nonce = response["result"].as_str().ok_or("Failed to get nonce from server")?; // with context

    
    let current_time = Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
    let domain = "localhost:4361"; // Your app's domain
    let uri = "http://localhost:4361"; // Your app's URI
    let chain_id = "11155111"; // Sepolia testnet
    let statement = "Garden.fi"; // Your app's statement

    let msg = format!(
        "{} wants you to sign in with your Ethereum account:\n\
        {}\n\n\
        {}\n\n\
        URI: {}\n\
        Version: 1\n\
        Chain ID: {}\n\
        Nonce: {}\n\
        Issued At: {}",
        domain,
        signer.address().to_string(),
        statement,
        uri,
        chain_id,
        nonce,
        current_time
    );
    
    // 5. Sign the message with your wallet
    let sig = signer.sign_message_sync(msg.as_bytes())?;
    let sig_hex = sig.as_bytes().encode_hex();


    // 6. Send the signature to the server to verify
    // Using serde_json::json! macro to create the JSON payload
    let payload = serde_json::json!({
        "message": msg,
        "signature": sig_hex,
        "nonce": nonce,
    });

    let res = client
        .post(format!("{}/auth/siwe/tokens", url))
        .json(&payload)
        .send().unwrap();
    
    
    let response: serde_json::Value = res.json().unwrap();    
    
    // 7. Get the JWT token from the response
    let jwt_token = response["result"].as_str().ok_or("Failed to authenticate with server")?.to_string();
    
    
    Ok(jwt_token)
}
