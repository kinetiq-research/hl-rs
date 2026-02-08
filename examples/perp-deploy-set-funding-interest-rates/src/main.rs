use std::str::FromStr;

use alloy::signers::local::PrivateKeySigner;
use hl_rs::{BaseUrl, ExchangeClient, SetFundingInterestRates};

#[tokio::main]
async fn main() {
    dotenv::dotenv().unwrap();

    let url = BaseUrl::Testnet;
    let dex_name = "dddd";

    // Set 8-hour funding interest rates for assets
    // Rates must be between -0.01 and 0.01
    let action = SetFundingInterestRates::new(dex_name, vec![
        ("BTC", 0.0001),   // 0.01% 8-hour interest rate
        ("ETH", -0.0001),  // -0.01% 8-hour interest rate
    ]);

    let private_key = std::env::var("PRIVATE_KEY").unwrap();
    let wallet = PrivateKeySigner::from_str(&private_key).unwrap();
    println!("wallet: {}", wallet.address());

    let client = ExchangeClient::new(url).with_signer(wallet);

    let result = client.send_action(action).await.unwrap();

    println!("Set funding interest rates result: {:?}", result);
}
