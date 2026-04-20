use std::str::FromStr;

use alloy::signers::local::PrivateKeySigner;
use hl_rs::{BaseUrl, ExchangeClient, RegisterToken};

#[tokio::main]
async fn main() {
    dotenv::dotenv().unwrap();

    let action = RegisterToken::new("MYTKN", 8, 18, 100_000, "My Test Token");

    let private_key = std::env::var("PRIVATE_KEY").unwrap();
    let wallet = PrivateKeySigner::from_str(&private_key).unwrap();
    println!("wallet: {}", wallet.address());

    let client = ExchangeClient::new(BaseUrl::Testnet).with_signer(wallet);
    let result = client.send_action(action).await.unwrap();
    println!("register_token: {:?}", result);
}
