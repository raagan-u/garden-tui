use std::str::FromStr;

use alloy::{hex::FromHex, primitives::{Address, FixedBytes, Uint}};
use bigdecimal::BigDecimal;
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
