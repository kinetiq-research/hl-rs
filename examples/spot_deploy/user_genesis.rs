use std::str::FromStr;

use alloy::signers::local::PrivateKeySigner;
use hl_rs::{BaseUrl, ExchangeClient, UserGenesis};

#[tokio::main]
async fn main() {
    dotenv::dotenv().unwrap();

    let token: u32 = std::env::var("SPOT_TOKEN")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(999);

    let recipient = std::env::var("GENESIS_USER").unwrap_or_else(|_| {
        let pk = std::env::var("PRIVATE_KEY").unwrap();
        let w = PrivateKeySigner::from_str(&pk).unwrap();
        w.address().to_string().to_lowercase()
    });

    let action = UserGenesis::new(
        token,
        vec![(recipient, "500000000000000000000000".to_string())],
        vec![],
    );

    let private_key = std::env::var("PRIVATE_KEY").unwrap();
    let wallet = PrivateKeySigner::from_str(&private_key).unwrap();
    println!("wallet: {}", wallet.address());

    let client = ExchangeClient::new(BaseUrl::Testnet).with_signer(wallet);
    let result = client.send_action(action).await.unwrap();
    println!("user_genesis: {:?}", result);
}
