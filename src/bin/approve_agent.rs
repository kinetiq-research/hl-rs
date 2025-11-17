// use std::{collections::HashMap, str::FromStr};

// use alloy::signers::local::PrivateKeySigner;
// use hl_rs::{BaseUrl, ExchangeClient};

#[tokio::main]
async fn main() {
    //     dotenv::dotenv().unwrap();

    //     let private_key = std::env::var("PRIVATE_KEY").unwrap();
    //     let wallet = PrivateKeySigner::from_str(&private_key).unwrap();

    //     let agent_private_key = std::env::var("AGENT_PRIVATE_KEY").unwrap();
    //     let agent_wallet = PrivateKeySigner::from_str(&agent_private_key).unwrap();

    //     let exchange_client =
    //         ExchangeClient::new(Some(BaseUrl::Testnet), None, None, HashMap::new()).unwrap();

    //     let approve_agent_action = exchange_client
    //         .approve_agent_action(agent_wallet.address(), "Example Agent")
    //         .unwrap()
    //         .sign(&wallet)
    //         .unwrap()
    //         .send()
    //         .await
    //         .unwrap();

    //     println!("Approve agent result: {approve_agent_action:?}");
}
