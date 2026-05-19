use std::str::FromStr;

use alloy::signers::local::PrivateKeySigner;
use hl_rs::{BaseUrl, EnableQuoteToken, ExchangeClient};

#[tokio::main]
async fn main() {
    dotenv::dotenv().unwrap();

    let token: u32 = std::env::var("SPOT_TOKEN")
        .ok()
        .and_then(|s| s.parse().ok())
        .expect("SPOT_TOKEN");

    let action = EnableQuoteToken::new(token);

    let private_key = std::env::var("PRIVATE_KEY").unwrap();
    let wallet = PrivateKeySigner::from_str(&private_key).unwrap();
    println!("wallet: {}", wallet.address());

    let client = ExchangeClient::new(BaseUrl::Testnet).with_signer(wallet);
    let result = client.send_action(action).await.unwrap();
    println!("enable_quote_token: {:?}", result);
}
