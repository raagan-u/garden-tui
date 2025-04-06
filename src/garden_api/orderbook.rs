use anyhow::{anyhow, Result};
use serde_json::Value;

use super::types::{MatchedOrder, Order};

pub struct Orderbook {
    client: reqwest::Client,
    url: String,
    api_key: String,
}

impl Orderbook {
    pub fn new(client: reqwest::Client, url: String, api_key: String) -> Self {
        Self { client , url, api_key }
    }

    pub async fn create_order(self, order : Order) -> Result<String> {
        let url = format!("{}/create-order", self.url);
        let resp = self.client.post(url).json(&order).send().await?;
        let result = resp.json::<Value>().await?;
        let create_id = result["result"].clone().to_string();
        Ok(create_id)
    }
    pub async fn initiate(self, order_id: &str, signature: &str) -> Result<String> {
        let url = format!("{}/initiate", self.url);
        let resp = self.client.post(url)
            .json(&serde_json::json!({
                "order_id": order_id,
                "signature": signature, 
                "perform_on": "Source"
            }))
            .send()
            .await?;
        let result = resp.json::<Value>().await?;
        Ok(result.to_string())
    }

    pub async fn redeem(self, order_id: &str, secret: &str) -> Result<String> {
        let url = format!("{}/redeem", self.url);
        let resp = self.client.post(url)
            .json(&serde_json::json!({
                "order_id": order_id,
                "secret": secret,
                "perform_on": "Destination"
            }))
            .send()
            .await?;
        let result = resp.json::<Value>().await?;
        Ok(result.to_string())
    }

    pub async fn get_matched_order(self, order_id: &str) -> Result<MatchedOrder> {
        let url = format!("{}/orders/id/matched/{}", self.url, order_id);
        let resp = self.client.get(url)
            .send()
            .await?;

        let response = resp.json::<Value>().await
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
