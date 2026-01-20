use std::str::FromStr;

use alloy::signers::local::PrivateKeySigner;
use hl_rs::{
    exchange::{
        builder::BuildAction,
        requests::{HaltTrading, PerpDeploy, SetMarginTableIds},
        ActionKind,
    },
    BaseUrl, ExchangeClient,
};

#[tokio::main]
async fn main() {
    dotenv::dotenv().unwrap();

    let private_key = std::env::var("PRIVATE_KEY").unwrap();
    let wallet = PrivateKeySigner::from_str(&private_key).unwrap();

    let url = BaseUrl::Testnet;
    let dex_name = "slob";
    let coins = vec!["TEST2"];
    let exchange_client = ExchangeClient::builder(url).build().await.unwrap();

    for coin in coins {
        let halt_trading = HaltTrading {
            coin: format!("{dex_name}:{coin}"),
            is_halted: false,
        };

        let signed_action = ActionKind::PerpDeploy(PerpDeploy::HaltTrading(halt_trading))
            .build(&exchange_client)
            .unwrap()
            .sign(&wallet)
            .unwrap();
        let halt_trading_result = exchange_client.send_action(signed_action).await;
        println!("Halt trading result for {coin}: {:?}", halt_trading_result);
    }
}
