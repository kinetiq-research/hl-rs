use std::str::FromStr;

use alloy::signers::local::PrivateKeySigner;
use hl_rs::{BaseUrl, ExchangeClient, RegisterSpot};

#[tokio::main]
async fn main() {
    dotenv::dotenv().unwrap();

    let base: u32 = std::env::var("BASE_TOKEN")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(999);
    let quote: u32 = std::env::var("QUOTE_TOKEN")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let action = RegisterSpot::new(base, quote);

    let private_key = std::env::var("PRIVATE_KEY").unwrap();
    let wallet = PrivateKeySigner::from_str(&private_key).unwrap();
    println!("wallet: {}", wallet.address());

    let client = ExchangeClient::new(BaseUrl::Testnet).with_signer(wallet);
    let result = client.send_action(action).await.unwrap();
    println!("register_spot: {:?}", result);
}
