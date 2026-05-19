use std::str::FromStr;

use alloy::signers::local::PrivateKeySigner;
use hl_rs::{BaseUrl, ExchangeClient, RegisterHyperliquidity};
use rust_decimal_macros::dec;

#[tokio::main]
async fn main() {
    dotenv::dotenv().unwrap();

    let spot: u32 = std::env::var("SPOT_MARKET")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(999);

    let action =
        RegisterHyperliquidity::new(spot, dec!(1.0), dec!(100.0), 10).with_seeded_levels(5);

    let private_key = std::env::var("PRIVATE_KEY").unwrap();
    let wallet = PrivateKeySigner::from_str(&private_key).unwrap();
    println!("wallet: {}", wallet.address());

    let client = ExchangeClient::new(BaseUrl::Testnet).with_signer(wallet);
    let result = client.send_action(action).await.unwrap();
    println!("register_hyperliquidity: {:?}", result);
}
