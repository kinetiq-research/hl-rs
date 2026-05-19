//! Link a Core spot token to an ERC20 on HyperEVM (`spotDeploy` → `requestEvmContract`).
//!
//! Set `EVM_CONTRACT` to the ERC20 address (hex). `EVM_EXTRA_WEI_DECIMALS` is typically the
//! difference between EVM and Core wei decimals (see Hyperliquid docs).

use std::str::FromStr;

use alloy::signers::local::PrivateKeySigner;
use hl_rs::{BaseUrl, ExchangeClient, RequestEvmContract};

#[tokio::main]
async fn main() {
    dotenv::dotenv().unwrap();

    let token: u32 = std::env::var("SPOT_TOKEN")
        .ok()
        .and_then(|s| s.parse().ok())
        .expect("SPOT_TOKEN");

    let contract = std::env::var("EVM_CONTRACT").expect("EVM_CONTRACT");
    let extra: i32 = std::env::var("EVM_EXTRA_WEI_DECIMALS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(13);

    let action = RequestEvmContract::new(token, contract.to_lowercase(), extra);

    let private_key = std::env::var("PRIVATE_KEY").unwrap();
    let wallet = PrivateKeySigner::from_str(&private_key).unwrap();
    println!("wallet: {}", wallet.address());

    let client = ExchangeClient::new(BaseUrl::Testnet).with_signer(wallet);
    let result = client.send_action(action).await.unwrap();
    println!("request_evm_contract: {:?}", result);
}
