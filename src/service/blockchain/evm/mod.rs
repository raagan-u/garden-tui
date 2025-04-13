use std::{convert::TryFrom, str::FromStr};
use alloy::{
    hex::FromHex, network::EthereumWallet, primitives::{Address, FixedBytes, Uint, U256}, providers::ProviderBuilder, signers::{local::PrivateKeySigner, Signature, Signer}, sol_types::eip712_domain
};
use reqwest::Url;

use crate::service::garden::types::SingleSwap;


alloy::sol!(
    #[sol(rpc)]
    GardenHTLC,
    "src/service/blockchain/evm/abi/htlc.json"
);

alloy::sol!(
    #[sol(rpc)]
    ERC20,
    "src/service/blockchain/evm/abi/erc20.json",
);

alloy::sol! {
    struct Initiate {
        address redeemer;
        uint256 timelock;
        uint256 amount;
        bytes32 secretHash;
    }
}



impl TryFrom<&SingleSwap> for Initiate {
    type Error = anyhow::Error;

    fn try_from(swap: &SingleSwap) -> Result<Self, Self::Error> {
        let redeemer = Address::from_hex(swap.redeemer.clone())
            .map_err(|e| anyhow::anyhow!("Failed to parse redeemer address: {}", e))?;

        let time_lock = Uint::from(swap.timelock);

        let amt = Uint::from_str(swap.amount.to_string().as_str())
            .map_err(|e| anyhow::anyhow!("Failed to parse amount: {}", e))?;

        let secret_hashbytes = FixedBytes::from_hex(&swap.secret_hash)
            .map_err(|e| anyhow::anyhow!("Failed to parse secret hash: {}", e))?;

        Ok(Initiate {
            redeemer,
            timelock: time_lock,
            amount: amt,
            secretHash: secret_hashbytes,
        })
    }
    
}

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
    
    
    
    let htlc_contract = GardenHTLC::new(
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

    let _erc20 = ERC20::new(token_address, provider.clone());

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