use anyhow::{anyhow, Result};
use bitcoin::{ScriptBuf, Script, opcodes};

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
