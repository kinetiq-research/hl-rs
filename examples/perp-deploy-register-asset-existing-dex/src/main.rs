use std::{collections::HashMap, str::FromStr};

use alloy::signers::local::PrivateKeySigner;
use hl_rs::{
    exchange::types::{DexParams, RegisterAssetParams},
    BaseUrl, ExchangeClient,
};

#[tokio::main]
async fn main() {
    dotenv::dotenv().unwrap();

    let agent_private_key = std::env::var("AGENT_PRIVATE_KEY").unwrap();
    let agent_wallet = PrivateKeySigner::from_str(&agent_private_key).unwrap();

    println!(
        "{}",
        format!(
            "Registering asset for agent with address {}",
            agent_wallet.address()
        )
    );

    let exchange_client = ExchangeClient::builder(BaseUrl::Testnet)
        .vault_address(agent_wallet.address())
        .build()
        .await
        .unwrap();

    let signed_action = exchange_client
        .register_asset_on_existing_dex(
            "EXAMPLE",
            RegisterAssetParams {
                max_gas: Some(1000000000000),
                ticker: "EXAMPLE:ASSET".to_string(),
                size_decimals: 2,
                oracle_price: 10.0,
                margin_table_id: 10,
                only_isolated: false,
            },
        )
        .unwrap()
        .sign(&agent_wallet)
        .unwrap();

    let payload = serde_json::to_string(&signed_action.action).unwrap();

    let register_asset_result = signed_action.send().await;

    println!("Register asset result: {:?}", register_asset_result);
}
