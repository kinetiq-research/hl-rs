use std::str::FromStr;

use alloy::signers::local::PrivateKeySigner;
use hl_rs::{BaseUrl, ExchangeClient, SetFundingInterestRates};
use rust_decimal_macros::dec;

#[tokio::main]
async fn main() {
    dotenv::dotenv().unwrap();

    let url = BaseUrl::Testnet;
    let dex_name = "dddd";

    // Option 1: Set 8-hour funding interest rates directly using Decimal
    // Rates must be between -0.01 and 0.01
    // NOTE: This is the baseline interest rate displayed if premium is 0 and funding scaling is 1
    let _action_direct = SetFundingInterestRates::new(
        dex_name,
        vec![
            ("TSLA", dec!(0.0001)), // 0.01% 8-hour interest rate
            ("ETH", dec!(-0.0001)), // -0.01% 8-hour interest rate
        ],
    );

    // Option 2: Set rate using target APY (more intuitive)
    // This converts a 5% APY to the corresponding 8-hour rate, given a 1.0x funding scale.
    let action = SetFundingInterestRates::from_target_apy(dex_name, "US500", 5.0);

    let private_key = std::env::var("PRIVATE_KEY").unwrap();
    let wallet = PrivateKeySigner::from_str(&private_key).unwrap();
    println!("wallet: {}", wallet.address());

    let client = ExchangeClient::new(url).with_signer(wallet);

    let result = client.send_action(action).await.unwrap();

    println!("Set funding interest rates result: {:?}", result);
}
