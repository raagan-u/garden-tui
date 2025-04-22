use std::{str::FromStr, time::Duration};

use alloy::signers::local::PrivateKeySigner;
use anyhow::{anyhow, Context, Result};
use garden_tui::service::{blockchain::evm::{init_and_get_sig, Initiate}, garden::{orderbook::Orderbook, quote::{generate_secret, Quote}, types::{self, InitiateRequest, Order, Strategy}}};

fn main() -> Result<()> {
    let client = reqwest::blocking::ClientBuilder::new().timeout(Duration::from_secs(10)).build()?;
    
    let quote_url = format!("http://testnet.api.garden.finance/quote");
    let quote_fetcher = Quote::new(client.clone(), quote_url)
        .context("Failed to create Quote fetcher")?;
    
    let mut strategy_id = String::new();
    let mut source_chain = String::new();
    let mut source_asset = None; // Will be replaced with actual value
    let mut dest_chain = String::new();
    let mut dest_asset = None; // Will be replaced with actual value
    let mut strategy = None;
    
    for (s_id, strat) in &quote_fetcher.strategies_map {
        if strat.source_chain == "arbitrum_sepolia" && strat.dest_chain == "bitcoin_testnet" {
            // Found the matching strategy
            strategy_id = s_id.to_string();
            source_chain = strat.source_chain.clone();
            source_asset = Some(strat.source_asset.clone());
            dest_chain = strat.dest_chain.clone();
            dest_asset = Some(strat.dest_asset.clone());
            strategy = Some(strat.clone());
            break; // Break out of the loop once we find the matching strategy
        }
    }
    
    // Check if we found a strategy
    let strategy = strategy.context("No matching strategy found")?;

    let order_pair = format!("{}:{}::{}:{}", 
        source_chain, 
        source_asset.unwrap().asset, 
        dest_chain, 
        dest_asset.unwrap().asset
    );
    let amount = "50000";
    
    let quote_price = quote_fetcher.get_price(&order_pair, amount)?;
    
    // You should set these to actual addresses
    let initiator_source_address = "YOUR_SOURCE_ADDRESS".to_string();
    let initiator_dest_address = "YOUR_DESTINATION_ADDRESS".to_string();
    
    let (secret_bytes, secret_hash_bytes) = generate_secret()?;
    let secret_hash = hex::encode(secret_hash_bytes);
    let secret = hex::encode(secret_bytes);
    
    let in_amount = amount.parse().unwrap();
    let out_amount = quote_price.parse().unwrap();
    
    // Replace with actual BTC recipient address if needed
    let btc_opt_recipient = Some("YOUR_BTC_RECIPIENT_ADDRESS".to_string());
    
    let order = Order::new(types::OrderInputData { 
        initiator_source_address, 
        initiator_dest_address, 
        in_amount, 
        out_amount, 
        secret_hash, 
        strategy, 
        btc_opt_recipient 
    });
    
    let attested_quote = quote_fetcher.get_attested_quote(order)?;
    
    println!("Order created successfully: {:?}", attested_quote);
    
    let eth_priv_key = "YOUR_ETH_PRIVATE_KEY".to_string();
    let signer = PrivateKeySigner::from_str(&eth_priv_key).expect("ERR CREATING ETH SIGNER");
    
    let orderbook = Orderbook::new(client, "RELAYER URL", &signer);
    
    let order_id = orderbook.create_order(attested_quote)?;
    
    println!("Order ID: {}", order_id);

     let matched_order = orderbook.get_matched_order(&order_id)?;
    
    let init_data = Initiate::try_from(&matched_order.source_swap).unwrap();
    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| anyhow!("Unable to create runtime: {}", e))
        .unwrap();
    
    let matched_order = orderbook.get_matched_order(&order_id)?;
    
    let (chain, _) = matched_order.source_swap.chain.split_once("_").unwrap();
    let rpc_url = format!("the rpc url of the respective chain");
    let signature = runtime
        .block_on(init_and_get_sig(init_data,"THE RESPECIVE CHAIN RPC URL", signer.clone(), &matched_order.source_swap.asset));

    let init_req = InitiateRequest{
        order_id: order_id.to_string(),
        signature: signature.to_string(),
        perform_on: "Source".to_string()
    };
    
    let tx = orderbook.initiate(init_req).unwrap();
    
    let destination_init_tx_hash = orderbook.wait_for_destination_init(&order_id);
    
    let redeem = orderbook.redeem(&order_id, &secret);
    
    Ok(())
}