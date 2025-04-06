use std::error::Error;

use alloy::{signers::{local::PrivateKeySigner, Signature}, sol_types::eip712_domain};
use anyhow::{anyhow, Result};
use bitcoin::{opcodes, Network, Script, ScriptBuf};
use rand::TryRngCore;
use reqwest::Client;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::str::FromStr;
use bitcoin::{
    absolute::LockTime, address::Address, consensus::encode::serialize_hex, key::{Keypair, Secp256k1}, secp256k1::Message, taproot::LeafVersion, transaction::Version, Amount, OutPoint, PrivateKey, Sequence, TapLeafHash, TapSighashType, Transaction, TxIn, TxOut, Txid, Witness
};
use bitcoin::sighash::SighashCache;

// use crate::garden_api::types::{AlloyProvider, Initiate};


#[derive(Debug, Deserialize)]
pub struct UTXO {
    pub txid: String,
    pub vout: u32,
    pub status: Status,
    pub value: u64,
}

#[derive(Debug, Deserialize)]
pub struct Status {
    pub confirmed: bool,
    pub block_height: u64,
    pub block_hash: String,
    pub block_time: u64,
}

pub async fn get_utxos(url: &str, address: &str) -> Result<Vec<UTXO>> {
    let client = Client::new();
    let url = url.to_string()+ "/address/"+address+"/utxo";
    println!("{:#?}", url);
    let resp = client.get(url).send().await?.json::<Vec<UTXO>>().await?;
    println!("sucess");
    Ok(resp)
}

pub fn redeem_leaf(secret_hash_bytes: &Vec<u8>, redeemer_pubkey: &str) -> Result<ScriptBuf> {
    if secret_hash_bytes.len() != 32 {
        return Err(anyhow!("Secret hash must be 32 bytes (64 hex chars), got {} bytes", secret_hash_bytes.len()));
    }
    
    let mut secret_hash_array = [0u8; 32];
    secret_hash_array.copy_from_slice(&secret_hash_bytes);
    
    
    let bytes = hex::decode(redeemer_pubkey)?;
    let mut redeem_pub_array = [0u8; 32];
    redeem_pub_array.copy_from_slice(&bytes[1..33]);
    
    let script = Script::builder()
        .push_opcode(opcodes::all::OP_SHA256)
        .push_slice(secret_hash_array)
        .push_opcode(opcodes::all::OP_EQUALVERIFY)
        .push_slice(&redeem_pub_array)
        .push_opcode(opcodes::all::OP_CHECKSIG)
        .into_script();
    
    Ok(script)
}

pub fn refund_leaf(timelock: i64, initiator_pubkey: &str) -> Result<ScriptBuf> {
    let bytes = hex::decode(&initiator_pubkey)?;
    let mut init_pub_array = [0u8; 32];
    init_pub_array.copy_from_slice(&bytes[1..33]);
    
    let script = Script::builder()
        .push_int(timelock)
        .push_opcode(opcodes::all::OP_CSV)
        .push_opcode(opcodes::all::OP_DROP)
        .push_slice(init_pub_array)
        .push_opcode(opcodes::all::OP_CHECKSIG)
        .into_script();
    
    Ok(script)
}

pub fn instant_refund_leaf(initiator_pubkey: &str, redeemer_pubkey: &str) -> Result<ScriptBuf> {
    let bytes = hex::decode(&initiator_pubkey)?;
    let mut init_pub_array = [0u8; 32];
    init_pub_array.copy_from_slice(&bytes[1..33]);
    
    let bytes = hex::decode(&redeemer_pubkey)?;
    let mut redeem_pub_array = [0u8; 32];
    redeem_pub_array.copy_from_slice(&bytes[1..33]);
    
    let script = Script::builder()
        .push_slice(&init_pub_array)
        .push_opcode(opcodes::all::OP_CHECKSIG)
        .push_slice(&redeem_pub_array)
        .push_opcode(opcodes::all::OP_CHECKSIGADD)
        .push_opcode(opcodes::all::OP_PUSHNUM_2)
        .push_opcode(opcodes::all::OP_NUMEQUAL)
        .into_script();
    
    Ok(script)
}

pub fn generate_secret() -> Result<([u8; 32], [u8; 32]), Box<dyn Error>> {
    let mut secret = [0u8; 32];
    
    rand::rng().try_fill_bytes(&mut secret)?;
    let mut hasher = Sha256::new();
    hasher.update(secret);
    let hash = hasher.finalize();

    let hash_bytes = hex::decode(hash)?;
    let mut hash_array = [0u8; 32];
    hash_array.copy_from_slice(&hash_bytes);
    
    Ok((secret, hash_array))
}


// Create a transaction for HTLC redemption with specified witness stack format
pub fn create_htlc_redeem_transaction(
    utxo_txid: &str,
    utxo_vout: u32,
    utxo_value: u64,
    receiver_address: &str,
    fee_rate: u64,
) -> Result<Transaction, Box<dyn std::error::Error>> {
    // Parse the UTXO transaction ID
    let txid = Txid::from_str(utxo_txid).unwrap();
    
    match Address::from_str(receiver_address) {
            Ok(receiver) => match receiver.require_network(bitcoin::Network::Testnet4) {
                Ok(btc_addr) => println!("Valid address: {:?}", btc_addr),
                Err(e) => println!("Network mismatch: {:?}", e),
            },
            Err(e) => println!("Invalid address format: {:?}", e),
        }
    let receiver = Address::from_str(receiver_address)?;
    let btc_addr = receiver.require_network(Network::Testnet4)?;
    
    // Calculate fee (approximately - we'll use a fixed size estimation)
    let estimated_tx_size = 200; // vbytes - adjust based on actual tx size
    let fee = fee_rate * estimated_tx_size;

    // Create output amount after deducting fee
    let output_value = utxo_value.saturating_sub(fee);

    // Create transaction
    let tx = Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint {
                txid,
                vout: utxo_vout,
            },
            script_sig: ScriptBuf::new(),
            sequence: Sequence(4294967294),
            witness: Witness::new(),
        }],
        output: vec![TxOut {
            value: Amount::from_sat(output_value),
            script_pubkey: btc_addr.script_pubkey(),
        }],
    };

    Ok(tx)
}

pub async fn create_tx(htlc_addr:Address, witness_stack: Vec<Vec<u8>>, receiver_address: &str, private_key_wif: &str) -> Result<Transaction> {
    let htlc_addr_string = htlc_addr.to_string();
    let utxos = get_utxos("https://mempool.space/testnet4/api", &htlc_addr_string).await?;
    if utxos.is_empty(){
        return Err(anyhow!("htlc address is not funded"))
    }
    
    let mut tx = create_htlc_redeem_transaction(
        &utxos[0].txid, 
        utxos[0].vout, 
        utxos[0].value,
        receiver_address, 
        3).unwrap();
    
    let leaf_hash = TapLeafHash::from_script(Script::from_bytes(&witness_stack[2].clone()), LeafVersion::TapScript);
    
    let mut prevouts: Vec<TxOut> = Vec::new();
    let htlc_pubkey = htlc_addr.script_pubkey();
    
    prevouts.push(
        TxOut { 
            value: Amount::from_sat(utxos[0].value), 
            script_pubkey:  htlc_pubkey
        }
    );
    
    tx = sign_and_set_taproot_witness(
        tx, 
        0, 
        witness_stack[2].clone(), 
        leaf_hash, 
        private_key_wif, 
        TapSighashType::All, 
        prevouts, 
        witness_stack[3].clone(), 
        witness_stack[1].clone()
    ).unwrap();
      
    println!("Transaction hex: {}", serialize_hex(&tx));
    
    Ok(tx)
}

pub fn sign_and_set_taproot_witness(
    mut tx: Transaction,
    input_index: usize,
    script_bytes: Vec<u8>,
    leaf_hash: TapLeafHash,
    wif: &str,
    sighash_type: TapSighashType,
    prevouts: Vec<bitcoin::TxOut>,
    control_block: Vec<u8>,
    secret_hash: Vec<u8>
) -> Result<Transaction, Box<dyn std::error::Error>> {
    
    let private_key = PrivateKey::from_wif(wif)?;
    
    let secp = Secp256k1::new();
    
    // Create keypair from private key
    let keypair = Keypair::from_secret_key(&secp, &private_key.inner);
    
    // Create sighash cache for the transaction
    let mut sighash_cache = SighashCache::new(&tx);
    
    // Generate the sighash message to sign using taproot script spend path
    let tap_sighash = sighash_cache.taproot_script_spend_signature_hash(
        input_index,
        &bitcoin::sighash::Prevouts::All(prevouts.as_slice()),
        leaf_hash,
        sighash_type,
    )?;
    
    // Convert TapSighash to a Message
    let message = Message::from_digest_slice(tap_sighash.as_ref())?;
    
    // Sign the sighash with Schnorr signature
    let signature = secp.sign_schnorr_no_aux_rand(&message, &keypair);
    
    let mut sig_serialized = signature.as_ref().to_vec();
    if sighash_type != TapSighashType::Default {
        sig_serialized.push(sighash_type as u8);
    }
    
    
    // Create the witness
    let mut witness = Witness::new();
    
    // Add the signature as the first element
    witness.push(sig_serialized);
    witness.push(secret_hash);
    witness.push(script_bytes);
    witness.push(control_block);
    
    tx.input[input_index].witness = witness;
    
    Ok(tx)
}

// pub async fn get_signature_for_init(init_data: Initiate, provider: AlloyProvider, signer:  PrivateKeySigner) -> Signature {
//     let chain = provider.get_chain_i.await.unwrap();
//     let (addr, network) = if chain == 31337 {
//         (
//             "9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0",
//             Network::,
//         )
//     } else if chain == 31338 {
//         (
//             "0165878A594ca255338adfa4d48449f69242Eb8F",
//             Network::Arbitrum,
//         )
//     } else {
//         panic!("chain not supported")
//     };
//     let htlc_contract = htlc::GardenHTLCContract::new(Address::from_hex(addr).unwrap(), provider.clone());
//     let d = htlc_contract
//         .eip712Domain()
//         .call()
//         .await.unwrap();
    
//     let domain = eip712_domain! {
//         name: d.name,
//         version: d.version,
//         chain_id: d.chainId.to(),
//         verifying_contract: d.verifyingContract,
//     };
    
//     let token_address = htlc_contract
//         .token()
//         .call()
//         .await
//         .expect("Failed to get token address")
//         ._0;

//     println!("Token address: {:?}", token_address);

//     let erc20 = htlc::ERC20Contract::new(token_address, provider.clone());

//     println!("{}", erc20.address().to_string());
//     let wallet_addr = Address::from_hex("0x6da99883352d5d3047e753667a62b06a78cd8e1c").unwrap();
//     let balance = provider.get_balance(wallet_addr).await.unwrap();
//     println!("Wallet balance: {}", balance);

//     erc20
//         .approve(*htlc_contract.address(), U256::MAX)
//         .send()
//         .await
//         .unwrap()
//         .watch()
//         .await
//         .unwrap();
//     let sig = signer.sign_typed_data(&init_data, &domain).await.unwrap();
//     sig
// }