use std::str::FromStr;

use alloy::signers::local::PrivateKeySigner;
use hl_rs::{BaseUrl, ExchangeClient, SetMarginTableIds};

#[tokio::main]
async fn main() {
    dotenv::dotenv().unwrap();

    let url = BaseUrl::Testnet;
    let dex_name = "dddd";
    let coin = "BOOF";
    let margin_table_id = 20; //20x max leverage cap

    // Set margin table IDs for assets
    // IDs reference margin tables previously inserted via InsertMarginTable
    let action = SetMarginTableIds::new(dex_name, vec![(coin, margin_table_id)]);

    let private_key = std::env::var("PRIVATE_KEY").unwrap();
    let wallet = PrivateKeySigner::from_str(&private_key).unwrap();
    println!("wallet: {}", wallet.address());

    let client = ExchangeClient::new(url).with_signer(wallet);

    let result = client.send_action(action).await.unwrap();

    println!("Set margin table IDs result: {:?}", result);
}
