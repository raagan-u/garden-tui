use std::error::Error;

use alloy::{
    hex::FromHex,
    primitives::U256,
    providers::Provider,
    signers::{local::PrivateKeySigner, Signature, Signer},
    sol_types::eip712_domain,
};
use anyhow::{anyhow, Result};
use bitcoin::sighash::SighashCache;
use bitcoin::{
    absolute::LockTime,
    address::Address,
    consensus::encode::serialize_hex,
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
use reqwest::Client;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::str::FromStr;

use crate::{
    garden_api::types::{AlloyProvider, Initiate},
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
    println!("{:#?}", url);

    let response = client.get(url).send().await?;
    let resp = response.json::<Vec<UTXO>>().await?;

    println!("sucess");
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

pub fn generate_secret() -> Result<([u8; 32], [u8; 32]), Box<dyn Error>> {
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
) -> Result<Transaction, Box<dyn std::error::Error>> {
    // Parse the UTXO transaction ID
    let txid = Txid::from_str(utxo_txid).unwrap();

    match Address::from_str(receiver_address) {
        Ok(receiver) => match receiver.require_network(bitcoin::Network::Regtest) {
            Ok(btc_addr) => println!("Valid address: {:?}", btc_addr),
            Err(e) => println!("Network mismatch: {:?}", e),
        },
        Err(e) => println!("Invalid address format: {:?}", e),
    }
    let receiver = Address::from_str(receiver_address)?;
    let btc_addr = receiver.require_network(Network::Regtest)?;

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
    receiver_address: &str,
    private_key_hex: &str,
) -> Result<Transaction> {
    let htlc_addr_string = htlc_addr.to_string();
    let utxos = get_utxos("http://0.0.0.0:30000", &htlc_addr_string).await?;
    if utxos.is_empty() {
        return Err(anyhow!("htlc address is not funded"));
    }

    let mut tx = create_htlc_redeem_transaction(
        &utxos[0].txid,
        utxos[0].vout,
        utxos[0].value,
        receiver_address,
        3,
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
        private_key_hex,
        TapSighashType::All,
        prevouts,
        witness_stack[3].clone(),
        witness_stack[1].clone(),
    )
    .unwrap();

    println!("Transaction hex: {}", serialize_hex(&tx));

    Ok(tx)
}

pub fn sign_and_set_taproot_witness(
    mut tx: Transaction,
    input_index: usize,
    script_bytes: Vec<u8>,
    leaf_hash: TapLeafHash,
    priv_key_hex: &str,
    sighash_type: TapSighashType,
    prevouts: Vec<bitcoin::TxOut>,
    control_block: Vec<u8>,
    secret_hash: Vec<u8>,
) -> Result<Transaction, Box<dyn std::error::Error>> {
    let priv_key_bytes = hex::decode(priv_key_hex).unwrap();
    let private_key =
        bitcoin::PrivateKey::from_slice(&priv_key_bytes, bitcoin::Network::Regtest).unwrap();

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

pub async fn init_and_get_sig(
    init_data: Initiate,
    provider: AlloyProvider,
    signer: PrivateKeySigner,
    token_address: &str,
) -> Signature {
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

    let balance = provider.get_balance(signer.address().clone());

    let erc20 = htlc::ERC20::new(token_address, provider.clone());

    erc20
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

pub fn pay_to_htlc(priv_key_hex: &str, htlc_addr: bitcoin::Address, amount: i64) -> Result<String> {
    // Decode private key and set up
    let priv_key_bytes = hex::decode(priv_key_hex)?;
    let secp = Secp256k1::new();
    let private_key = PrivateKey::from_slice(&priv_key_bytes, bitcoin::Network::Regtest).unwrap();
    let public_key = PublicKey::from_private_key(&secp, &private_key);
    let compressed_pubkey = CompressedPublicKey::try_from(public_key).unwrap();
    let sender_address = Address::p2wpkh(&compressed_pubkey, Network::Regtest);

    // Set up runtime for async calls
    let url = "http://0.0.0.0:30000";
    let runtime =
        tokio::runtime::Runtime::new().map_err(|e| anyhow!("Unable to create runtime: {}", e))?;

    // Get and filter UTXOs
    let utxos = runtime.block_on(get_utxos(url, &sender_address.to_string()))?;
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
        let compressed_pubkey = CompressedPublicKey::try_from(public_key)?;
        let change_addr = Address::p2wpkh(&compressed_pubkey, Network::Regtest);
        outputs.push(TxOut {
            value: Amount::from_sat(total_input - amount as u64 - fee),
            script_pubkey: change_addr.script_pubkey(),
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

    let tx_id = submit_tx(url, &signed_tx);
    tx_id
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_pay_to_htlc() {
        // Test private key (regtest)
        let private_key = "e331b6d69882b4c45e512e0ea84e3c837e6b0fabe57b8c6c0b6a9a7ed2e5c8e3";

        // Generate address from private key for regtest
        let priv_key_bytes = hex::decode(private_key).unwrap();
        let secp = Secp256k1::new();
        let private_key_obj =
            PrivateKey::from_slice(&priv_key_bytes, bitcoin::Network::Regtest).unwrap();
        let public_key = PublicKey::from_private_key(&secp, &private_key_obj);
        let compressed_pubkey = CompressedPublicKey::try_from(public_key).unwrap();
        let sender_address = Address::p2wpkh(&compressed_pubkey, Network::Regtest);

        println!("Fund this address to test: {}", sender_address);

        // Create a test HTLC address
        let htlc_addr = Address::from_str("bcrt1qw508d6qejxtdg4y5r3zarvary0c5xw7kygt080").unwrap();
        let checked = htlc_addr
            .require_network(bitcoin::Network::Regtest)
            .unwrap();
        // Test amount
        let amount = 100000; // 0.001 BTC in satoshis

        // Call the function
        let result = pay_to_htlc(private_key, checked, amount);

        println!("{:#?}", result);

        // Check if the result is Ok (this will fail in actual testing since we need real UTXOs)
        assert!(result.is_ok());

        // The actual test would need a local Bitcoin node with known UTXOs
        // and would verify the transaction structure, amounts, etc.
    }
}
