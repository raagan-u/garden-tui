use anyhow::{anyhow, Result};
use serde_json::Value;

use super::types::{InitiateRequest, MatchedOrder, Order};

#[derive(Clone)]
pub struct Orderbook {
    client: reqwest::blocking::Client,
    url: String,
    api_key: String,
}

impl Orderbook {
    pub fn new(client: reqwest::blocking::Client, url: String, api_key: String) -> Self {
        Self { client , url, api_key }
    }

    pub fn create_order(&self, order: Order) -> Result<String> {
        let url = format!("{}/relayer/create-order", self.url);
        let resp = self.client
            .post(url)
            .header("api-key", &self.api_key)
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
        Ok(create_id)
    }
    pub fn initiate(&self, init_req: InitiateRequest) -> Result<String> {
        let url = format!("{}/relayer/initiate", self.url);
        let resp = self.client.post(url)
            .header("api-key", &self.api_key)
            .json(&init_req)
            .send()?;
        let result = resp.json::<Value>()?;
        Ok(result.to_string())
    }
    pub fn wait_for_destination_init(&mut self, order_id: &str) -> Result<String> {
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
        }
    }
    
    pub fn redeem(&self, order_id: &str, secret: &str) -> Result<String> {
        let url = format!("{}/relayer/redeem", self.url);
        let resp = self.client.post(url).header("api-key", &self.api_key)
            .json(&serde_json::json!({
                "order_id": order_id,
                "secret": secret,
                "perform_on": "Destination"
            }))
            .send()?;
        let result = resp.json::<Value>()?;
        Ok(result.to_string())
    }

    pub fn get_matched_order(&self, order_id: &str) -> Result<MatchedOrder> {
        let url = format!("{}/orders/id/matched/{}", self.url, order_id);
        let resp = self.client
            .get(url)
            .header("api-key", &self.api_key)
            .send()?;

        let response = resp.json::<Value>()
            .map_err(|e| anyhow!("error getting matched order {}", e))?;

        // Extract the order from the nested response structure
        let result = response.get("result")
            .ok_or_else(|| anyhow!("missing result field in response"))?;

        // Parse the order value into an Order struct
        let order: MatchedOrder = serde_json::from_value(result.clone())
            .map_err(|e| anyhow!("failed to parse order: {}", e))?;

        Ok(order)
    }
}
