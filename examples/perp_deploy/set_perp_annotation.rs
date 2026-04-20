use std::str::FromStr;

use alloy::signers::local::PrivateKeySigner;
use hl_rs::{BaseUrl, ExchangeClient, SetPerpAnnotationBuilder};

#[tokio::main]
async fn main() {
    dotenv::dotenv().unwrap();

    let url = BaseUrl::Mainnet;

    // For a DEX-deployed perp (formats coin as "dex:SYMBOL"):
    // let action = SetPerpAnnotation::new("mydex", "BTC", "Crypto", "Bitcoin perpetual contract");

    // For a DEX-deployed perp, use ::new which formats coin as "dex:SYMBOL":
    let action = SetPerpAnnotationBuilder::new("km", "XIAOMI")
        .category("stocks")
        .description("Xiaomi Corporation. Chinese consumer electronics and smart device manufacturer engaged in the design, development, and sale of smartphones, IoT devices, and lifestyle products, underpinned by an internet services ecosystem spanning advertising, fintech, and smart home platforms. Primary oracle pricing reference: 1810.HK")
        .build()
        .unwrap();
    //    "km",
    //    "USTECH",
    //    "indices",
    //    "Provides exposure to 100 of the largest non-financial companies listed on U.S. exchanges, with a concentration in technology, communications, and high-growth sectors. Please refer to km docs for further info on oracle reference pricing sources and design.",
    //).display_name(Some("USTECH100"));

    let private_key = std::env::var("PRIVATE_KEY").unwrap();
    let wallet = PrivateKeySigner::from_str(&private_key).unwrap();
    println!("wallet: {}", wallet.address());

    let client = ExchangeClient::new(url).with_signer(wallet);

    let result = client.send_action(action).await.unwrap();

    println!("Set perp annotation result: {:?}", result);
}
