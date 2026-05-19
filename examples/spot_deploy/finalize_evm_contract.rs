//! Finalize Core ↔ EVM token linking after `request_evm_contract`.
//!
//! - `FINALIZE_MODE=create` (default): uses the EOA deployment tx nonce (`DEPLOY_NONCE`).
//! - `FINALIZE_MODE=first_slot`: `FinalizeEvmContract::first_storage_slot(token)`.
//! - `FINALIZE_MODE=custom_slot`: `FinalizeEvmContract::custom_storage_slot(token)`.

use std::str::FromStr;

use alloy::signers::local::PrivateKeySigner;
use hl_rs::{BaseUrl, ExchangeClient, FinalizeEvmContract};

#[tokio::main]
async fn main() {
    dotenv::dotenv().unwrap();

    let token: u32 = std::env::var("SPOT_TOKEN")
        .ok()
        .and_then(|s| s.parse().ok())
        .expect("SPOT_TOKEN");

    let mode = std::env::var("FINALIZE_MODE").unwrap_or_else(|_| "create".to_string());

    let action = match mode.as_str() {
        "first_slot" => FinalizeEvmContract::first_storage_slot(token),
        "custom_slot" => FinalizeEvmContract::custom_storage_slot(token),
        _ => {
            let nonce: u64 = std::env::var("DEPLOY_NONCE")
                .ok()
                .and_then(|s| s.parse().ok())
                .expect("DEPLOY_NONCE for FINALIZE_MODE=create");
            FinalizeEvmContract::by_create_nonce(token, nonce)
        }
    };

    let private_key = std::env::var("PRIVATE_KEY").unwrap();
    let wallet = PrivateKeySigner::from_str(&private_key).unwrap();
    println!("wallet: {}", wallet.address());

    let client = ExchangeClient::new(BaseUrl::Testnet).with_signer(wallet);
    let result = client.send_action(action).await.unwrap();
    println!("finalize_evm_contract: {:?}", result);
}
