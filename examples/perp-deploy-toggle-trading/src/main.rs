use std::str::FromStr;

use alloy::signers::local::PrivateKeySigner;
use hl_rs::{BaseUrl, ExchangeClient, ToggleTrading};

#[tokio::main]
async fn main() {
    dotenv::dotenv().unwrap();

    let url = BaseUrl::Testnet;
    let dex_name = "dddd";

    // Resume (enable) trading for an asset
    let action = ToggleTrading::resume(dex_name, "TSLA");

    // To halt (disable) trading instead, use:
    // let action = ToggleTrading::halt(dex_name, "TSLA");

    let private_key = std::env::var("PRIVATE_KEY").unwrap();
    let wallet = PrivateKeySigner::from_str(&private_key).unwrap();
    println!("wallet: {}", wallet.address());

    let client = ExchangeClient::new(url).with_signer(wallet);

    let result = client.send_action(action).await.unwrap();

    println!("Toggle trading result: {:?}", result);
}
