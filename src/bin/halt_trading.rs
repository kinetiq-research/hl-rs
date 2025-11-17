// use std::{collections::HashMap, str::FromStr};

// use alloy::signers::local::PrivateKeySigner;
// use hl_rs::{BaseUrl, ExchangeClient};

#[tokio::main]
async fn main() {
    // dotenv::dotenv().unwrap();

    // let agent_private_key = std::env::var("AGENT_PRIVATE_KEY").unwrap();
    // let agent_wallet = PrivateKeySigner::from_str(&agent_private_key).unwrap();

    // let exchange_client =
    //     ExchangeClient::new(Some(BaseUrl::Testnet), None, None, HashMap::new()).unwrap();

    // let perp_deploy_action = exchange_client
    //     .halt_trading_action("BTC", true)
    //     .unwrap()
    //     .sign(&agent_wallet)
    //     .unwrap()
    //     .send()
    //     .await
    //     .unwrap();

    // println!("Halt trading result: {perp_deploy_action:?}");
}
