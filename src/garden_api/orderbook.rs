use anyhow::{anyhow, Result};
use serde_json::Value;

use super::types::{MatchedOrder, Order};

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

    pub fn create_order(self, order: Order) -> Result<String> {
        let url = format!("{}/relayer/create-order", self.url);
        let resp = self.client
            .post(url)
            .header("api-key", self.api_key)
            .json(&order)
            .send()
            .map_err(|e| anyhow!("failed to send create order request: {}", e))?;

        let result = resp.json::<Value>()
            .map_err(|e| anyhow!("failed to parse response as JSON: {}", e))?;
        
        
        let create_id = result.get("result")
            .ok_or_else(|| anyhow!("missing result field in response: {}", serde_json::to_string_pretty(&result).unwrap()))?
            .clone()
            .to_string();

        Ok(create_id)
    }
    pub fn initiate(self, order_id: &str, signature: &str) -> Result<String> {
        let url = format!("{}/initiate", self.url);
        let resp = self.client.post(url)
            .json(&serde_json::json!({
                "order_id": order_id,
                "signature": signature, 
                "perform_on": "Source"
            }))
            .send()?;
        let result = resp.json::<Value>()?;
        Ok(result.to_string())
    }

    pub fn redeem(self, order_id: &str, secret: &str) -> Result<String> {
        let url = format!("{}/redeem", self.url);
        let resp = self.client.post(url)
            .json(&serde_json::json!({
                "order_id": order_id,
                "secret": secret,
                "perform_on": "Destination"
            }))
            .send()?;
        let result = resp.json::<Value>()?;
        Ok(result.to_string())
    }

    pub fn get_matched_order(self, order_id: &str) -> Result<MatchedOrder> {
        let url = format!("{}/orders/id/matched/{}", self.url, order_id);
        let resp = self.client.get(url)
            .send()?;

        let response = resp.json::<Value>()
            .map_err(|e| anyhow!("error getting matched order {}", e))?;

        // Extract the order from the nested response structure
        let result = response.get("result")
            .ok_or_else(|| anyhow!("missing result field in response"))?;

        // Parse the order value into an Order struct
        let attested_order: MatchedOrder = serde_json::from_value(result.clone())
            .map_err(|e| anyhow!("failed to parse order: {}", e))?;

        Ok(attested_order)
    }
}
