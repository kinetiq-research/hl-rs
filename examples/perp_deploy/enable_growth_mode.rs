use std::str::FromStr;

use alloy::signers::local::PrivateKeySigner;
use hl_rs::{BaseUrl, ExchangeClient, SetGrowthModes};

#[tokio::main]
async fn main() {
    dotenv::dotenv().unwrap();

    let url = BaseUrl::Testnet;
    let dex_name = "dddd";

    // Enable growth mode for specific assets
    // Growth mode status can only be changed once every 30 days
    let action = SetGrowthModes::new(
        dex_name,
        vec![
            ("US500", true), // Enable growth mode for TSLA
        ],
    );

    let private_key = std::env::var("PRIVATE_KEY").unwrap();
    let wallet = PrivateKeySigner::from_str(&private_key).unwrap();
    println!("wallet: {}", wallet.address());

    let client = ExchangeClient::new(url).with_signer(wallet);

    let result = client.send_action(action).await.unwrap();

    println!("Set growth modes result: {:?}", result);
}
