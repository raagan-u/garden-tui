use anyhow::{anyhow, Result};
use bitcoin::{
    key::Secp256k1, secp256k1::{self, PublicKey, XOnlyPublicKey}, taproot::{LeafVersion, TaprootBuilder}, Address, KnownHrp, Network, ScriptBuf
};

use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

use super::scripts::{redeem_leaf, refund_leaf, instant_refund_leaf};

pub fn garden_nums() -> Result<XOnlyPublicKey, Box<dyn std::error::Error>> {
    let mut hasher = Sha256::new();
    hasher.update(b"GardenHTLC");
    let r = hasher.finalize();

    // Parse the BIP-341 H point
    let h_hex = "0250929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0";
    let h = PublicKey::from_slice(&hex::decode(h_hex)?)?;

    // Create the r*G point
    let secp = Secp256k1::new();
    let r_scalar = secp256k1::SecretKey::from_slice(&r)?;
    let r_g = PublicKey::from_secret_key(&secp, &r_scalar);

    // Add H + r*G
    let nums = h.combine(&r_g)?;

    let (xonly, _) = nums.x_only_public_key();

    Ok(xonly)
}

pub struct BitcoinHTLC {
    initiator_pubkey: String,
    redeemer_pubkey: String,
    secret_hash: Vec<u8>,
    timelock: i64,
    network: Network,
}

impl BitcoinHTLC {
    pub fn new(
        secret_hash: Vec<u8>,
        initiator_pubkey: String,
        redeemer_pubkey: String,
        timelock: i64,
        network: Network
    ) -> Result<Self> {
        Ok(Self {
            initiator_pubkey,
            redeemer_pubkey,
            secret_hash,
            timelock,
            network
        })
    }

    fn construct_taproot(&self) -> Result<TaprootBuilder> {
        let redeem_leaf = redeem_leaf(&self.secret_hash, &self.redeemer_pubkey).expect("error building redeem leaf");
        let refund_leaf = refund_leaf(self.timelock, &self.initiator_pubkey).expect("error building refund leaf");

        let instant_refund = instant_refund_leaf(&self.initiator_pubkey, &self.redeemer_pubkey).expect("error building instand refund leaf");

        let mut script_map = BTreeMap::new();
        script_map.insert(10, redeem_leaf);
        script_map.insert(5, refund_leaf);
        script_map.insert(1, instant_refund);

        let taproot = TaprootBuilder::with_huffman_tree(script_map)
            .map_err(|e| anyhow!("Failed to create huffman tree: {}", e))?;

        Ok(taproot)
    }

    pub fn address(&self) -> Result<Address> {
        let secp = Secp256k1::new();

        let taproot_builder = self.construct_taproot().expect("error building taproot tree");

        if !taproot_builder.is_finalizable() {
            return Err(anyhow::anyhow!("Taproot builder is not finalizable"));
        }

        let internal_key =
            garden_nums().map_err(|e| anyhow!("error creating internal_key {}", e)).expect("error getting garden NUMS");
        
        let spend_info = taproot_builder.finalize(&secp, internal_key).expect("error finalizing builder");
        let addr = Address::p2tr(
            &secp,
            internal_key,
            spend_info.merkle_root(),
            KnownHrp::from(self.network),
        );
        Ok(addr)
    }
    
    pub fn get_control_block(&self, leaf: Leaf) -> Result<(ScriptBuf, Vec<u8>)> {
        let secp = Secp256k1::new();
        let internal_key = garden_nums().unwrap();
        let taproot_script_tree = self.construct_taproot()?.finalize(&secp, internal_key).unwrap();
        
        let (leaf_script, cb_bytes) = match leaf {
            Leaf::Redeem => {
                let redeem = redeem_leaf(&self.secret_hash, &self.redeemer_pubkey)?;
                
                let ctrlblck = taproot_script_tree.control_block(&(redeem.clone(), LeafVersion::TapScript)).unwrap();
                
                let cb_bytes = ctrlblck.serialize();
                (redeem, cb_bytes.clone())
            },
            Leaf::Refund => {
                let refund = refund_leaf(self.timelock, &self.redeemer_pubkey)?;
                
                let ctrlblck = taproot_script_tree.control_block(&(refund.clone(), LeafVersion::TapScript)).unwrap();
                
                let cb_bytes = ctrlblck.serialize();
                (refund, cb_bytes.clone())
            },
            Leaf::InstantRefund => {
                let instant_refund = instant_refund_leaf(&self.initiator_pubkey, &self.redeemer_pubkey)?;
                
                let ctrlblck = taproot_script_tree.control_block(&(instant_refund.clone(), LeafVersion::TapScript)).unwrap();
                
                let cb_bytes = ctrlblck.serialize();
                (instant_refund, cb_bytes.clone())
            }
        };
        Ok((leaf_script, cb_bytes))
    }
    
    pub fn redeem(&self, secret: &Vec<u8>) -> Result<Vec<Vec<u8>>> {
        let mut hasher = Sha256::new();
        hasher.update(secret);
        let secret_hash_bytes = hasher.finalize().to_vec();
    
        if !secret_hash_bytes.eq(&self.secret_hash) {
            return Err(anyhow!("secret mismatch")); 
        }
        
        
        let (redeem_script, cb_bytes) = self.get_control_block(Leaf::Redeem)?;
        let sig_data = hex::decode("000000000000")?;
        
        let mut witness_data: Vec<Vec<u8>> = Vec::new();
        
        witness_data.extend([
            sig_data,
            secret.clone(),
            redeem_script.into_bytes(),
            cb_bytes,
        ]);
        Ok(witness_data)
    }
    
    #[allow(dead_code)]
    pub fn refund(&self) -> Result<Vec<Vec<u8>>> {
        let mut witness_data: Vec<Vec<u8>> = Vec::new();
        let sig_data = hex::decode("000000000000")?;
        
        let (refund_script, cb_bytes) = self.get_control_block(Leaf::Refund)?;
        
        witness_data.extend([
            sig_data,
            refund_script.into_bytes(),
            cb_bytes,
        ]);
        
        Ok(witness_data)
    }
    
    #[allow(dead_code)]
    pub fn instant_refund(&self) -> Result<Vec<Vec<u8>>> {
        let mut witness_data: Vec<Vec<u8>> = Vec::new();
        let sig_data = hex::decode("000000000000")?;
        let random_sig = hex::decode("1111111111111")?;
        let (instant_refund_script, cb_bytes) = self.get_control_block(Leaf::InstantRefund)?;
        
        witness_data.extend([
            sig_data,
            random_sig,
            instant_refund_script.into_bytes(),
            cb_bytes,
        ]);
        
        Ok(witness_data)
    }
}

pub enum Leaf {
    Redeem,
    Refund,
    InstantRefund
}