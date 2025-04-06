use std::collections::HashMap;

use anyhow::{anyhow, Result};
use serde_json::Value;
use super::types::{Order, Strategy};

pub struct Quote {
    client: reqwest::blocking::Client,
    url: String,
    pub strategies_map: Option<HashMap<String, Strategy>>,
}

impl Quote {
    pub fn new(client: reqwest::blocking::Client, url: String) -> Self {
        Self { client, url , strategies_map: None }
    }
    
    // pub async fn fetch_strategies(&self) -> Result<Value>{
    //     let url = format!("{}/quote/strategies", self.url);
    //     let resp = self.client.get(&url).send().await?;
    //     let response = resp.json::<Value>().await.map_err(|e| anyhow!("error fetching strategies from quote {}", e))?;
    //     let strategies = response["result"].clone();
    //     Ok(strategies)
    // }
    
    pub fn load_strategies(&mut self) -> Result<bool>{
        let url = format!("{}/quote/strategies", self.url);
        let resp = self.client.get(&url).send()?;
        let response = resp.json::<Value>().map_err(|e| anyhow!("error fetching strategies from quote {}", e))?;
        let strategies: HashMap<String, Strategy> = serde_json::from_value(response["result"].clone())
            .map_err(|e| anyhow!("error deserializing strategies: {}", e))?;
        if !strategies.is_empty() {
            self.strategies_map = Some(strategies.clone())
        }
        Ok(!strategies.is_empty())
    }
    
    pub  fn get_price(&self, order_pair: &str, amount: &str) -> Result<String> {
        let url = format!("{}/quote?order_pair={}&amount={}&exact_out={}", self.url, order_pair, amount, true);
        let resp = self.client.get(&url).send()?;
        let response = resp.json::<Value>().map_err(|e| anyhow!("error getting quote price {}", e))?;
        println!("response: {:?}", response);
        Ok(response.to_string())
    }
    
    pub  fn get_attested_quote(&self, order: Order) -> Result<Order> {
        let url = format!("{}/quote/attested", self.url);
        let resp = self.client.post(&url).json(&order).send()?;
        
        let response = resp.json::<Value>()
            .map_err(|e| anyhow!("error getting attested quote {}", e))?;
        
        // Extract the order from the nested response structure
        let result = response.get("result")
            .ok_or_else(|| anyhow!("missing result field in response"))?;
            
        // Parse the order value into an Order struct
        let attested_order: Order = serde_json::from_value(result.clone())
            .map_err(|e| anyhow!("failed to parse order: {}", e))?;
        
        Ok(attested_order)
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use bigdecimal::BigDecimal;

//     #[tokio::test]
//     async fn test_fetch_strategies() {
//         let quote = Quote::new(reqwest::Client::new(), "https://quote-staging.hashira.io".to_string());
//         let strategies = quote.fetch_strategies().await.unwrap();
//         println!("{:?}", strategies);
//         assert!(strategies.is_object());
//     }

//     #[tokio::test]
//     async fn test_get_price() {
//         let quote = Quote::new(reqwest::Client::new(), "https://quote-staging.hashira.io".to_string());
//         let price = quote.get_price("arbitrum_sepolia:0x795Dcb58d1cd4789169D5F938Ea05E17ecEB68cA::bitcoin_testnet:primary", "10000").await.unwrap();
//         println!("price: {}", price);
//         assert!(price.contains("\"status\":\"Ok"));
//     }

//     #[tokio::test]
//     async fn test_get_attested_quote() {
//         let quote = Quote::new(reqwest::Client::new(), "https://quote-staging.hashira.io".to_string());
//         let order = Order {
//             source_chain: "arbitrum_sepolia".to_string(),
//             destination_chain: "bitcoin_testnet".to_string(),
//             source_asset: "0x795Dcb58d1cd4789169D5F938Ea05E17ecEB68cA"
//                 .to_string()
//                 .to_lowercase(),
//             destination_asset: "primary".to_string(),
//             initiator_source_address: "0x70997c70c51812dc3a010c7d01b50e0d17dc79c8"
//                 .to_string(),
//             initiator_destination_address: "1db36714896afaee20c2cc817d170689870858b5204d3b5a94d217654e94b2fb"
//                 .to_string(),
//             source_amount: BigDecimal::from(10000),
//             destination_amount: BigDecimal::from(1000),
//             fee: BigDecimal::from(1000),
//             nonce: BigDecimal::from(1),
//             min_destination_confirmations: 1,
//             timelock: 600,
//             secret_hash: "acea7af1f0c8b96f84548bbce0488b08cdaf2c02b23579b4abc5945155d93722".to_string(),
//             additional_data: crate::garden_api::types::AdditionalData {
//                 strategy_id: "asacbtyr".to_string(),
//                 bitcoin_optional_recipient: None,
//                 input_token_price: None,
//                 output_token_price: None,
//                 sig: None,
//                 deadline: None,
//             },
//         };
//         let attested_quote = quote.get_attested_quote(order).await.unwrap();
//         println!("attested_quote: {:?}", attested_quote);
//         assert!(attested_quote.additional_data.sig.is_some());
//     }
    
//     #[tokio::test]
//     async fn test_load_strategies() {
//         let mut quote = Quote::new(reqwest::Client::new(), "https://quote-staging.hashira.io".to_string());
//         let result = quote.load_strategies().await.unwrap();
//         println!("{:#?}", quote.strategies_map.unwrap().get("asacbtyr"));
//         assert_eq!(result, true)
//     }
// }
