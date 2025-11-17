use std::{collections::HashMap, str::FromStr};

use alloy::signers::local::PrivateKeySigner;
use hl_rs::{BaseUrl, ExchangeClient, exchange::requests::PerpDexSchemaInput};

const REGISTER_PERP_DEX: bool = true;
const DUMMY_PERP_DEX: &str = "dummy";

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

    let exchange_client = ExchangeClient::new(
        Some(BaseUrl::Testnet),
        Some(agent_wallet.address()),
        None,
        HashMap::new(),
    )
    .unwrap();

    let perp_dex_schema_input = if REGISTER_PERP_DEX {
        Some(PerpDexSchemaInput {
            full_name: "test dex".to_string(),
            collateral_token: 0,
            oracle_updater: Some(agent_wallet.address().to_string().to_lowercase()),
        })
    } else {
        None
    };

    let signed_action = exchange_client
        .register_asset_action(
            Some(1000000000000),
            format!("{DUMMY_PERP_DEX}:DUMMY"),
            2,
            10.0,
            10,
            false,
            DUMMY_PERP_DEX.to_string(),
            perp_dex_schema_input,
        )
        .unwrap()
        .sign(&agent_wallet)
        .unwrap();

    let payload = serde_json::to_string(&signed_action.action).unwrap();

    let register_asset_result = signed_action.send().await;

    println!("Register asset result: {:?}", register_asset_result);
}
