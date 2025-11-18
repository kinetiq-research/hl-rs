use std::{collections::HashMap, str::FromStr};

use alloy::signers::local::PrivateKeySigner;
use hl_rs::{
    exchange::types::{ExternalPerpPrice, MarkPrice, OraclePrice, SetOracleParams},
    BaseUrl, ExchangeClient,
};

#[tokio::main]
async fn main() {
    dotenv::dotenv().unwrap();

    let agent_private_key = std::env::var("AGENT_PRIVATE_KEY").unwrap();
    let agent_wallet = PrivateKeySigner::from_str(&agent_private_key).unwrap();

    let exchange_client = ExchangeClient::builder(BaseUrl::Testnet)
        .vault_address(agent_wallet.address())
        .build()
        .await
        .unwrap();

    let signed_action = exchange_client
        .set_oracle(SetOracleParams {
            dex_name: "dummy".to_string(),
            oracle_prices: vec![OraclePrice {
                asset: "EXAMPLE:ASSET".to_string(),
                price: "10.0".to_string(),
            }],
            mark_prices: vec![vec![MarkPrice {
                asset: "EXAMPLE:ASSET".to_string(),
                price: "10.0".to_string(),
            }]],
            external_perp_prices: vec![ExternalPerpPrice {
                asset: "EXAMPLE:ASSET".to_string(),
                price: "10.0".to_string(),
            }],
        })
        .unwrap()
        .sign(&agent_wallet)
        .unwrap();

    let payload = serde_json::to_string(&signed_action.action).unwrap();

    let register_asset_result = signed_action.send().await;

    println!("Register asset result: {:?}", register_asset_result);
}
