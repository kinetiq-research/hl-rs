use std::str::FromStr;

use alloy::signers::local::PrivateKeySigner;
use hl_rs::{BaseUrl, ExchangeClient, SetFundingClamps};
use rust_decimal_macros::dec;

#[tokio::main]
async fn main() {
    dotenv::dotenv().unwrap();

    let url = BaseUrl::Testnet;
    let dex_name = "bart";

    // 8-hour funding clamps, must be between 0 and 0.01 (default 0.0003).
    // Bounds how far funding can move away from the average premium toward the interest rate.
    let action = SetFundingClamps::new(dex_name, vec![("AAPL", dec!(0.0030))]);

    let private_key = std::env::var("PRIVATE_KEY").unwrap();
    let wallet = PrivateKeySigner::from_str(&private_key).unwrap();
    println!("wallet: {}", wallet.address());

    let client = ExchangeClient::new(url).with_signer(wallet);

    let result = client.send_action(action).await.unwrap();

    println!("Set funding clamps result: {:?}", result);
}
