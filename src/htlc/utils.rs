use alloy::{
    hex::FromHex, network::{EthereumWallet, NetworkWallet}, primitives::U256, providers::{Provider, ProviderBuilder}, signers::{local::PrivateKeySigner, Signature, Signer}, sol_types::eip712_domain
};
use anyhow::{anyhow, Result};
use bitcoin::sighash::SighashCache;
use bitcoin::{
    absolute::LockTime,
    address::Address,
    key::{Keypair, Secp256k1},
    secp256k1::Message,
    taproot::LeafVersion,
    transaction::Version,
    Amount, OutPoint, PrivateKey, Sequence, TapLeafHash, TapSighashType, Transaction, TxIn, TxOut,
    Txid, Witness,
};
use bitcoin::{
    hex::DisplayHex, opcodes, CompressedPublicKey, EcdsaSighashType, Network, PublicKey, Script,
    ScriptBuf,
};
use rand::TryRngCore;
use ratatui::text::ToLine;
use reqwest::{Client, Url};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::str::FromStr;

use crate::{
    garden_api::types::Initiate,
    htlc,
};

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct UTXO {
    pub txid: String,
    pub vout: u32,
    pub status: Status,
    pub value: u64,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Status {
    pub confirmed: bool,
    #[serde(default)]
    pub block_height: u64,
    #[serde(default)]
    pub block_hash: String,
    #[serde(default)]
    pub block_time: u64,
}

pub async fn get_utxos(url: &str, address: &str) -> Result<Vec<UTXO>> {
    let client = Client::new();
    let url = url.to_string() + "/address/" + address + "/utxo";

    let response = client.get(url).send().await?;
    let resp = response.json::<Vec<UTXO>>().await?;

    Ok(resp)
}

pub fn redeem_leaf(secret_hash_bytes: &Vec<u8>, redeemer_pubkey: &str) -> Result<ScriptBuf> {
    if secret_hash_bytes.len() != 32 {
        return Err(anyhow!(
            "Secret hash must be 32 bytes (64 hex chars), got {} bytes",
            secret_hash_bytes.len()
        ));
    }

    let mut secret_hash_array = [0u8; 32];
    secret_hash_array.copy_from_slice(&secret_hash_bytes);

    let bytes = hex::decode(redeemer_pubkey)?;
    let mut redeem_pub_array = [0u8; 32];
    redeem_pub_array.copy_from_slice(&bytes[0..32]);

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
    init_pub_array.copy_from_slice(&bytes[0..32]);

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
    init_pub_array.copy_from_slice(&bytes[0..32]);

    let bytes = hex::decode(&redeemer_pubkey)?;
    let mut redeem_pub_array = [0u8; 32];
    redeem_pub_array.copy_from_slice(&bytes[0..32]);

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

// Create a transaction for HTLC redemption with specified witness stack format
pub fn create_htlc_redeem_transaction(
    utxo_txid: &str,
    utxo_vout: u32,
    utxo_value: u64,
    receiver_address: &str,
    fee_rate: u64,
    network: bitcoin::Network
) -> Result<Transaction, Box<dyn std::error::Error>> {
    // Parse the UTXO transaction ID
    let txid = Txid::from_str(utxo_txid).unwrap();
    // Parse the BTC address, handling possible errors
    let btc_addr = match Address::from_str(receiver_address) {
        Ok(receiver) => match receiver.require_network(network) {
            Ok(addr) => addr,
            Err(e) => return Err(format!("Network mismatch: {:?}", e).into()),
        },
        Err(e) => return Err(format!("Invalid address format: {:?}", e).into()),
    };

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

pub async fn create_tx(
    htlc_addr: Address,
    witness_stack: Vec<Vec<u8>>,
    receiver_address: Option<String>,
    private_key_hex: &str,
    network: bitcoin::Network
) -> Result<Transaction> {
    let private_key = PrivateKey::from_slice(&hex::decode(private_key_hex)?, network)?;
    let htlc_addr_string = htlc_addr.to_string();
    let utxos = get_utxos("http://0.0.0.0:30000", &htlc_addr_string).await?;
    if utxos.is_empty() {
        return Err(anyhow!("htlc address is not funded"));
    }

    let recipient = match receiver_address {
        Some(addr) => addr,
        None => {
            let address = get_btc_address_for_priv_key(private_key, network)?;
            address   
        }
    };
    
    let mut tx = create_htlc_redeem_transaction(
        &utxos[0].txid,
        utxos[0].vout,
        utxos[0].value,
        &recipient,
        3,
        network
    )
    .unwrap();

    let leaf_hash = TapLeafHash::from_script(
        Script::from_bytes(&witness_stack[2].clone()),
        LeafVersion::TapScript,
    );

    let mut prevouts: Vec<TxOut> = Vec::new();
    let htlc_pubkey = htlc_addr.script_pubkey();

    prevouts.push(TxOut {
        value: Amount::from_sat(utxos[0].value),
        script_pubkey: htlc_pubkey,
    });

    tx = sign_and_set_taproot_witness(
        tx,
        0,
        witness_stack[2].clone(),
        leaf_hash,
        private_key,
        TapSighashType::All,
        prevouts,
        witness_stack[3].clone(),
        witness_stack[1].clone(),
    )
    .unwrap();

    

    Ok(tx)
}

pub fn get_btc_address_for_priv_key(private_key: PrivateKey, network: bitcoin::Network) -> Result<String> {
    let secp = Secp256k1::new();
    let public_key = PublicKey::from_private_key(&secp, &private_key);
    let compressed_pubkey = CompressedPublicKey::try_from(public_key)?;
    let addr = Address::p2wpkh(&compressed_pubkey, network).to_string();
    Ok(addr)
}

pub fn sign_and_set_taproot_witness(
    mut tx: Transaction,
    input_index: usize,
    script_bytes: Vec<u8>,
    leaf_hash: TapLeafHash,
    private_key: bitcoin::PrivateKey,
    sighash_type: TapSighashType,
    prevouts: Vec<bitcoin::TxOut>,
    control_block: Vec<u8>,
    secret_hash: Vec<u8>,
) -> Result<Transaction, Box<dyn std::error::Error>> {
    // Create keypair from private key
    let secp = Secp256k1::new();
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
/// Initializes an HTLC interaction and obtains a signature
/// 
/// This function performs several steps:
/// 1. Creates a new HTLC contract instance using the provided token address
/// 2. Retrieves the EIP-712 domain information for typed data signing
/// 3. Gets the token address from the HTLC contract
/// 4. Creates an ERC20 contract instance
/// 5. Approves the HTLC contract to spend the maximum amount of tokens
/// 6. Signs the initiation data with the domain information
///
/// Returns a typed data signature needed for HTLC initialization
pub async fn init_and_get_sig(
    init_data: Initiate,
    provider_url: &str,
    signer: PrivateKeySigner,
    token_address: &str,
) -> Signature {
    
    let eth_wallet = EthereumWallet::new(signer.clone());
    
    let provider_url = Url::from_str(&provider_url).unwrap();
    
    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(eth_wallet)
        .on_http(provider_url);
    
    
    
    let htlc_contract = htlc::GardenHTLC::new(
        alloy::primitives::Address::from_hex(token_address).unwrap(),
        provider.clone(),
    );
    let d = htlc_contract.eip712Domain().call().await.unwrap();

    let domain = eip712_domain! {
        name: d.name,
        version: d.version,
        chain_id: d.chainId.to(),
        verifying_contract: d.verifyingContract,
    };

    let token_address = htlc_contract
        .token()
        .call()
        .await
        .expect("Failed to get token address")
        ._0;

    let _erc20 = htlc::ERC20::new(token_address, provider.clone());

    _erc20
        .approve(*htlc_contract.address(), U256::MAX)
        .send()
        .await
        .unwrap()
        .watch()
        .await
        .unwrap();
    let sig = signer.sign_typed_data(&init_data, &domain).await.unwrap();
    sig
}

pub fn filter_for_amount(utxos: Vec<UTXO>, amount: i64) -> Result<Vec<UTXO>> {
    let mut filtered_utxos: Vec<UTXO> = Vec::new();
    let mut total = 0;

    for utxo in utxos {
        total += utxo.value as i64;
        filtered_utxos.push(utxo);
        if total == amount {
            return Ok(filtered_utxos);
        }
    }

    if total < amount {
        return Err(anyhow!("Not enough funds in UTXOs"));
    }
    Ok(filtered_utxos)
}

pub fn submit_tx(url: &str, tx: &bitcoin::Transaction) -> Result<String> {
    let endpoint = format!("{}/tx", url);
    let client = reqwest::blocking::Client::new();
    let tx_bytes = bitcoin::consensus::serialize(tx);
    let hex_tx = hex::encode(tx_bytes);
    let str_buffer = hex_tx.as_bytes();

    let resp = client
        .post(&endpoint)
        .header("Content-Type", "application/text")
        .body(str_buffer.to_vec())
        .send()
        .map_err(|e| e)?;

    if !resp.status().is_success() {
        let err_msg = resp.text().map_err(|e| e)?;
        return Err(anyhow!("req failed : {:#?}", err_msg));
    }

    Ok(resp.text()?.to_string())
}

pub fn pay_to_htlc(priv_key_hex: &str, htlc_addr: bitcoin::Address, amount: i64, indexer_url: &str, network: bitcoin::Network) -> Result<String> {
    // Decode private key and set up
    let priv_key_bytes = hex::decode(priv_key_hex)?;
    let secp = Secp256k1::new();
    let private_key = PrivateKey::from_slice(&priv_key_bytes, network).unwrap();
    let public_key = PublicKey::from_private_key(&secp, &private_key);
    let compressed_pubkey = CompressedPublicKey::try_from(public_key).unwrap();
    let sender_address = Address::p2wpkh(&compressed_pubkey, network);

    let runtime =
        tokio::runtime::Runtime::new().map_err(|e| anyhow!("Unable to create runtime: {}", e))?;

    // Get and filter UTXOs
    let utxos = runtime.block_on(get_utxos(indexer_url, &sender_address.to_string()))?;
    let filtered_utxos = filter_for_amount(utxos, amount)?;

    // Create inputs and track values
    let mut inputs: Vec<TxIn> = Vec::new();
    let mut input_values: Vec<u64> = Vec::new();
    for utxo in filtered_utxos {
        let txid = Txid::from_str(&utxo.txid)?;
        inputs.push(TxIn {
            previous_output: OutPoint {
                txid,
                vout: utxo.vout,
            },
            script_sig: ScriptBuf::new(),
            sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
            witness: Witness::new(),
        });
        input_values.push(utxo.value);
    }

    // Calculate fee and total input amount
    let fee = 250 * inputs.len() as u64;
    let total_input: u64 = input_values.iter().sum();

    // Parse HTLC address and create outputs
    let output = TxOut {
        value: Amount::from_sat(amount as u64),
        script_pubkey: htlc_addr.script_pubkey(),
    };

    let mut outputs = vec![output];

    // Add change output if needed
    if total_input > (amount as u64 + fee) {
        outputs.push(TxOut {
            value: Amount::from_sat(total_input - amount as u64 - fee),
            script_pubkey: sender_address.script_pubkey(),
        });
    }

    // Create unsigned transaction
    let mut unsigned_tx = Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: inputs,
        output: outputs,
    };

    // Sign each input
    let mut sighash_cache = SighashCache::new(&mut unsigned_tx);

    for i in 0..input_values.len() {
        // Create the script for this input (p2wpkh)
        let script_pubkey = ScriptBuf::new_p2wpkh(&public_key.wpubkey_hash()?);

        // Get the sighash to sign
        let sighash_type = EcdsaSighashType::All;
        let sighash = sighash_cache.p2wpkh_signature_hash(
            i,
            &script_pubkey,
            Amount::from_sat(input_values[i]),
            sighash_type,
        )?;

        // Sign the sighash
        let msg = Message::from(sighash);
        let signature = secp.sign_ecdsa(&msg, &private_key.inner);

        // Create the signature with sighash type
        let btc_signature = bitcoin::ecdsa::Signature {
            signature,
            sighash_type,
        };
        let pubkey_bytes = public_key.to_bytes();
        *sighash_cache.witness_mut(i).unwrap() = Witness::p2wpkh(
            &btc_signature,
            &bitcoin::secp256k1::PublicKey::from_slice(&pubkey_bytes)?,
        )
    }

    // Get the fully signed transaction
    let signed_tx = sighash_cache.into_transaction();

    let tx_id = submit_tx(indexer_url, &signed_tx);
    tx_id
}