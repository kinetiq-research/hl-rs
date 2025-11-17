// use std::{collections::HashMap, str::FromStr};

// use alloy::{primitives::Address, signers::local::PrivateKeySigner};
// use hl_rs::{BaseUrl, ExchangeClient};

#[tokio::main]
async fn main() {
    // dotenv::dotenv().unwrap();

    // let private_key = std::env::var("PRIVATE_KEY").unwrap();
    // let wallet = PrivateKeySigner::from_str(&private_key).unwrap();

    // println!(
    //     "{}",
    //     format!(
    //         "Sending USDC from {} to 0x1234567890123456789012345678901234567890",
    //         wallet.address()
    //     )
    // );

    // let exchange_client =
    //     ExchangeClient::new(Some(BaseUrl::Testnet), None, None, HashMap::new()).unwrap();

    // let action = exchange_client
    //     .usdc_transfer_action(
    //         "100",
    //         Address::from_str("0x1234567890123456789012345678901234567890").unwrap(),
    //     )
    //     .unwrap()
    //     .sign(&wallet)
    //     .unwrap()
    //     .send()
    //     .await
    //     .unwrap();

    // println!("USDC transfer result: {action:?}");
}
