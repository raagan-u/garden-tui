use anyhow::{anyhow, Context, Result};
use bitcoin::{
    absolute::LockTime, address::Address, key::{Keypair, Secp256k1}, secp256k1::{All, Message}, sighash::SighashCache, taproot::LeafVersion, transaction::Version, Amount, CompressedPublicKey, EcdsaSighashType, OutPoint, PrivateKey, PublicKey, Script, ScriptBuf, Sequence, TapLeafHash, TapSighashType, Transaction, TxIn, TxOut, Txid, Witness
};

use serde::Deserialize;
use std::{str::FromStr, time::Duration};

pub struct SimpleIndexer {
    client: reqwest::Client,
    url: String
}

impl SimpleIndexer {
    pub fn new(url: &str) -> Result<Self> {
        let client = reqwest::ClientBuilder::new()
            .timeout(Duration::from_secs(5))
            .build()?;
        
        Ok(
            Self { client, url: url.to_string() }
        )
    }
    
    pub async fn get_utxos(&self, address: &str) -> Result<Vec<UTXO>> {
        let url = format!("{}/address/{}/utxo", &self.url, address);
    
        let response = self.client.get(url).send().await?;
        let resp = response.json::<Vec<UTXO>>().await?;
    
        Ok(resp)
    }

    pub async fn get_utxos_for_amount(&self, address:&str, amount: i64) -> Result<Vec<UTXO>> {
        let utxos = self.get_utxos(address).await?;
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
    
    pub async fn submit_tx(&self, tx: &bitcoin::Transaction) -> Result<String> {
        let endpoint = format!("{}/tx", self.url);
        let tx_bytes = bitcoin::consensus::serialize(tx);
        let hex_tx = hex::encode(tx_bytes);
        let str_buffer = hex_tx.as_bytes();
    
        let resp = self.client
            .post(&endpoint)
            .header("Content-Type", "application/text")
            .body(str_buffer.to_vec())
            .send().await
            .map_err(|e| e)?;
    
        if !resp.status().is_success() {
            let err_msg = resp.text().await.map_err(|e| e)?;
            return Err(anyhow!("req failed : {:#?}", err_msg));
        }
    
        Ok(resp.text().await?.to_string())
    }

}


pub struct HtlcHandler {
    network: bitcoin::Network,
    indexer: SimpleIndexer,
    secp: Secp256k1<All>
}

impl HtlcHandler {
    pub fn new(network: bitcoin::Network, indexer_url: &str) -> Result<Self> {
        Ok(Self{
            network,
            indexer: SimpleIndexer::new(indexer_url)?,
            secp: Secp256k1::new(),
        })
    }
    
    pub async fn broadcast_tx(&self, tx: &Transaction) -> Result<String> {
        let tx_id = self.indexer.submit_tx(tx).await.context("failed to broadcast transaction")?;
        Ok(tx_id)
    }
    
    pub fn get_btc_address_for_priv_key(&self, private_key: PrivateKey) -> Result<String> {
        let public_key = PublicKey::from_private_key(&self.secp, &private_key);
        let compressed_pubkey = CompressedPublicKey::try_from(public_key)?;
        let addr = Address::p2wpkh(&compressed_pubkey, self.network).to_string();
        Ok(addr)
    }

    
    pub fn initaite_htlc(&self, private_key: PrivateKey, htlc_addr: bitcoin::Address, amount: i64) -> Result<Transaction> {
        let public_key = PublicKey::from_private_key(&self.secp, &private_key);
        let compressed_pubkey = CompressedPublicKey::try_from(public_key).unwrap();
        let sender_address = Address::p2wpkh(&compressed_pubkey, self.network);
    
        let runtime =
            tokio::runtime::Runtime::new().map_err(|e| anyhow!("Unable to create runtime: {}", e))?;
    
        let utxos = runtime.block_on(self.indexer.get_utxos_for_amount(&sender_address.to_string(), amount))?;
    
        // Create inputs and track values
        let mut inputs: Vec<TxIn> = Vec::new();
        let mut input_values: Vec<u64> = Vec::new();
        for utxo in utxos {
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
            let signature = self.secp.sign_ecdsa(&msg, &private_key.inner);
    
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
    
        let signed_tx = sighash_cache.into_transaction();
        
        Ok(signed_tx.clone())
    }
    
    pub async fn create_redeem_tx(
        &self,
        htlc_addr: Address,
        witness_stack: Vec<Vec<u8>>,
        receiver_address: Option<String>,
        private_key: PrivateKey,
        fee_rate: u64,
    ) -> Result<Transaction> {
        // Determine the recipient address
        let recipient = match receiver_address {
            Some(addr) => addr,
            None => self.get_btc_address_for_priv_key(private_key)?
        };
        
        // Fetch UTXOs for the HTLC address
        let htlc_addr_string = htlc_addr.to_string();
        let utxos = self.indexer.get_utxos(&htlc_addr_string).await?;
        if utxos.is_empty() {
            return Err(anyhow!("htlc address is not funded"));
        }
        let utxo = &utxos[0];
        
        // Parse the UTXO transaction ID
        let txid = Txid::from_str(&utxo.txid).unwrap();
        
        // Parse the BTC address
        let btc_addr = Address::from_str(&recipient)
            .map_err(|e| anyhow!("Invalid address format: {:?}", e))?
            .require_network(self.network)
            .map_err(|e| anyhow!("Network mismatch: {:?}", e))?;
        
        // Calculate fee
        let estimated_tx_size = 200; // vbytes - adjust based on actual tx size
        let fee = fee_rate * estimated_tx_size;
        
        // Create output amount after deducting fee
        let output_value = utxo.value.saturating_sub(fee);
        
        // Create the unsigned transaction
        let mut tx = Transaction {
            version: Version::TWO,
            lock_time: LockTime::ZERO,
            input: vec![TxIn {
                previous_output: OutPoint {
                    txid,
                    vout: utxo.vout,
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
        
        // Prepare for signing
        let leaf_hash = TapLeafHash::from_script(
            Script::from_bytes(&witness_stack[2]),
            LeafVersion::TapScript,
        );
        
        // Create prevouts for signing
        let mut prevouts = Vec::new();
        prevouts.push(TxOut {
            value: Amount::from_sat(utxo.value),
            script_pubkey: htlc_addr.script_pubkey(),
        });
        
        // Sign the transaction
        tx = self.sign_and_set_taproot_witness(
            tx,
            0,
            leaf_hash,
            private_key,
            TapSighashType::All,
            prevouts,
            witness_stack
        )?;
        
        Ok(tx)
    }
    
    pub fn sign_and_set_taproot_witness(
        &self,
        mut tx: Transaction,
        input_index: usize,
        leaf_hash: TapLeafHash,
        private_key: bitcoin::PrivateKey,
        sighash_type: TapSighashType,
        prevouts: Vec<bitcoin::TxOut>,
        witness_stack: Vec<Vec<u8>>
    ) -> Result<Transaction> {
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
        witness.push(&witness_stack[1]);
        witness.push(&witness_stack[2]);
        witness.push(&witness_stack[3]);
    
        tx.input[input_index].witness = witness;
    
        Ok(tx)
    }
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct UTXO {
    pub txid: String,
    pub vout: u32,
    pub status: Status,
    pub value: u64,
}

#[derive(Debug, Deserialize, Clone)]
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

