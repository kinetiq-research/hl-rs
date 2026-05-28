//! Place a limit order on the perpetuals exchange.
//!
//! Uses ETH (asset index 4 on testnet) by default. Set ASSET_INDEX in .env to override.

use std::str::FromStr;

use alloy::signers::local::PrivateKeySigner;
use hl_rs::{BaseUrl, BatchOrder, ExchangeClient, LimitOrderType, OrderType, OrderWire, Tif};
use rust_decimal_macros::dec;

#[tokio::main]
async fn main() {
    dotenv::dotenv().unwrap();

    let url = BaseUrl::Testnet;

    // Add 10_000 to market index for spot
    let asset: u32 = 10_000 + 1338; // USDH/USDC Spot Testnet

    let order = OrderWire {
        asset,
        is_buy: false,
        limit_px: dec!(1),
        size: dec!(11.0),
        reduce_only: false,
        order_type: OrderType::Limit(LimitOrderType { tif: Tif::Gtc }),
        client_order_id: None,
    };

    let action = BatchOrder::new(vec![order]);

    let private_key = std::env::var("PRIVATE_KEY").unwrap();
    let wallet = PrivateKeySigner::from_str(&private_key).unwrap();
    println!("wallet: {}", wallet.address());

    let client = ExchangeClient::new(url).with_signer(wallet);

    let result = client.send_action(action).await.unwrap();

    println!("Place order result: {:?}", result);
}
