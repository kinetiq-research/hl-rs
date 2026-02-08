use std::str::FromStr;

use alloy::signers::local::PrivateKeySigner;
use hl_rs::{BaseUrl, ExchangeClient, SetOpenInterestCaps};

#[tokio::main]
async fn main() {
    dotenv::dotenv().unwrap();

    let url = BaseUrl::Testnet;
    let dex_name = "dddd";

    // Set open interest caps for multiple assets at once
    // Caps must be at least max(1_000_000, half of current OI)
    let action = SetOpenInterestCaps::new(dex_name, vec![
        ("TSLA", 10_000_000 * 100_000),  // $10M cap for TSLA
    ]);

    let private_key = std::env::var("PRIVATE_KEY").unwrap();
    let wallet = PrivateKeySigner::from_str(&private_key).unwrap();
    println!("wallet: {}", wallet.address());

    let client = ExchangeClient::new(url).with_signer(wallet);

    let result = client.send_action(action).await.unwrap();

    println!("Set OI caps result: {:?}", result);
}
