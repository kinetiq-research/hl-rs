use std::str::FromStr;

use alloy::signers::local::PrivateKeySigner;
use hl_rs::{actions::RegisterAsset, AssetRequest, BaseUrl, ExchangeClient};
use rust_decimal_macros::dec;

#[tokio::main]
async fn main() {
    dotenv::dotenv().unwrap();

    let url = BaseUrl::Mainnet;
    let dex_name = "km";

    let asset_request = AssetRequest {
        coin: "MU".to_string(),
        sz_decimals: 3,
        oracle_px: dec!(411.66),
        margin_table_id: 10,
        only_isolated: true,
    };

    let action = RegisterAsset::new(dex_name, asset_request);

    let private_key = std::env::var("PRIVATE_KEY").unwrap();
    let wallet = PrivateKeySigner::from_str(&private_key).unwrap();
    println!("wallet: {}", wallet.address());

    let client = ExchangeClient::new(url).with_signer(wallet);

    let register_asset_result = client.send_action(action).await.unwrap();

    println!("Register asset result: {:?}", register_asset_result);
}
