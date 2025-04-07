pub mod bitcoin_htlc;
pub mod utils;

alloy::sol!(
    #[sol(rpc)]
    GardenHTLC,
    "src/htlc/abi/htlc.json"
);

alloy::sol!(
    #[sol(rpc)]
    ERC20,
    "src/htlc/abi/erc20.json",
);