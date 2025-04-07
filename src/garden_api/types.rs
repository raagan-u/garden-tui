use std::str::FromStr;

use alloy::{hex::FromHex, primitives::{Address, FixedBytes, Uint}};
use bigdecimal::{BigDecimal, FromPrimitive};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdditionalData {
    pub strategy_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bitcoin_optional_recipient: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_token_price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_token_price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sig: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deadline: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Order {
    pub source_chain: String,
    pub destination_chain: String,
    pub source_asset: String,
    pub destination_asset: String,
    pub initiator_source_address: String,
    pub initiator_destination_address: String,
    pub source_amount: BigDecimal,
    pub destination_amount: BigDecimal,
    pub fee: BigDecimal,
    pub nonce: BigDecimal,
    pub min_destination_confirmations: u64,
    pub timelock: u64,
    pub secret_hash: String,
    pub additional_data: AdditionalData,
}

pub struct OrderInputData {
    pub initiator_source_address: String,
    pub in_amount: u64,
    pub out_amount: u64,
    pub secret_hash: String,
    pub strategy: Strategy,
    pub btc_opt_recepient: Option<String>
}

impl Order {
    pub fn new(order_data: OrderInputData) -> Self {
        Self { 
            source_chain: order_data.strategy.source_chain, 
            destination_chain: order_data.strategy.dest_chain, 
            source_asset: order_data.strategy.source_asset.asset, 
            destination_asset: order_data.strategy.dest_asset.asset, 
            initiator_source_address: order_data.initiator_source_address, 
            initiator_destination_address: order_data.strategy.dest_chain_address, 
            source_amount: BigDecimal::from_u64(order_data.in_amount).unwrap(), 
            destination_amount: BigDecimal::from_u64(order_data.out_amount).unwrap(), 
            fee: BigDecimal::from_u64(order_data.strategy.fee).unwrap(), 
            nonce: BigDecimal::from_u64(100).unwrap(), 
            min_destination_confirmations: 1, 
            timelock: order_data.strategy.min_source_timelock, 
            secret_hash: order_data.secret_hash, 
            additional_data: AdditionalData { 
                strategy_id: order_data.strategy.id, 
                bitcoin_optional_recipient: order_data.btc_opt_recepient, 
                input_token_price: None, 
                output_token_price: None, 
                sig: None, 
                deadline: None 
            } 
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub asset: String,
    pub token_id: String,
    pub decimals: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Strategy {
    pub id: String,
    pub source_chain_address: String,
    pub dest_chain_address: String,
    pub source_chain: String,
    pub dest_chain: String,
    pub source_asset: Asset,
    pub dest_asset: Asset,
    pub makers: Vec<String>,
    pub min_amount: BigDecimal,
    pub max_amount: BigDecimal,
    pub min_source_timelock: u64,
    pub min_source_confirmations: u64,
    pub min_price: f64,
    pub fee: u64, // in bips
}

alloy::sol! {
    struct Initiate {
        address redeemer;
        uint256 timelock;
        uint256 amount;
        bytes32 secretHash;
    }
}

impl Order {
    pub fn to_sol_initiate(&self, redeemer_addr: &str) -> Initiate {
        let redeemer = Address::from_hex(redeemer_addr).unwrap();
        let time_lock = Uint::from(self.timelock);
        let amt = Uint::from_str(self.source_amount.to_string().as_str()).unwrap();
        let secret_hashbytes = FixedBytes::from_hex(self.secret_hash.clone()).unwrap();
        Initiate {
            redeemer,
            timelock: time_lock,
            amount: amt,
            secretHash: secret_hashbytes,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SingleSwap {
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
    pub swap_id: String,
    pub chain: String,
    pub asset: String,
    pub initiator: String,
    pub redeemer: String,
    pub timelock: i32,
    pub filled_amount: BigDecimal,
    pub amount: BigDecimal,
    pub secret_hash: String,
    pub secret: Option<String>,
    pub initiate_tx_hash: Option<String>,
    pub redeem_tx_hash: Option<String>,
    pub refund_tx_hash: Option<String>,
    pub initiate_block_number: Option<BigDecimal>,
    pub redeem_block_number: Option<BigDecimal>,
    pub refund_block_number: Option<BigDecimal>,
    pub required_confirmations: i32,
    pub current_confirmations: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchedOrder {
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
    pub source_swap: SingleSwap,
    pub destination_swap: SingleSwap,
    pub create_order: Order,
}
