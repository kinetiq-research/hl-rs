use std::env;

use alloy::primitives::Address;
use alloy::signers::local::PrivateKeySigner;
use rust_decimal_macros::dec;
use serde::Serialize;

use hl_rs::exchange::action_v2::{Action, ExchangeActionV2Client, ToggleBigBlocks, UsdSend, SetOpenInterestCaps};
use hl_rs::BaseUrl;

fn load_private_key() -> PrivateKeySigner {
    //let key = env::var("HL_PRIVATE_KEY")
    //    .expect("HL_PRIVATE_KEY must be set to run this integration test");
    let key = "0x03066e580c00d884fbac56956e2cf2f5ab4941b5de03e4d7c6884724bd54c40e".to_owned();
    key.parse::<PrivateKeySigner>()
        .expect("HL_PRIVATE_KEY must be a valid private key")
}

async fn test_send_action<T: Action + Serialize + std::fmt::Debug>(
    action: T,
) -> Result<hl_rs::exchange::responses::ExchangeResponse, hl_rs::Error> {
    let client = ExchangeActionV2Client::new(BaseUrl::Testnet).with_signer(load_private_key());
    client.send_action(action).await
}

#[tokio::test]
async fn usd_send_int_test() {
    let usd_send = UsdSend::builder()
        .destination(Address::repeat_byte(0x11))
        .amount(dec!(1.69))
        .build()
        .unwrap();

    println!("Signer address: {}", load_private_key().address());
    let response = test_send_action(usd_send).await.unwrap();
    println!("usd_send response: {:#?}", response);
}

#[tokio::test]
async fn toggle_big_blocks_int_test() {
    let toggle = ToggleBigBlocks::enable();
    let response = test_send_action(toggle).await.unwrap();
    println!("toggle_big_blocks response: {:#?}", response);
}

#[tokio::test]
async fn set_open_interest_caps_int_test() {
    let set_open_interest_caps = SetOpenInterestCaps::new(
        "slob",
        vec![
            ("TEST0", 10 * 1_000_000),
        ]
    );
    let response = test_send_action(set_open_interest_caps).await.unwrap();
    println!("set_open_interest_caps response: {:#?}", response);
}

