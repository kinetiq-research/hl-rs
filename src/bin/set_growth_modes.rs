use std::str::FromStr;

use alloy::signers::local::PrivateKeySigner;
use hl_rs::{
    exchange::{
        builder::BuildAction,
        requests::{PerpDeploy, SetGrowthModes},
        ActionKind,
    },
    BaseUrl, ExchangeClient,
};

#[tokio::main]
async fn main() {
    dotenv::dotenv().unwrap();

    let private_key = std::env::var("PRIVATE_KEY").unwrap();
    let wallet = PrivateKeySigner::from_str(&private_key).unwrap();

    let dex_name = "km";
    let coins = vec!["TSLA"];
    let url = BaseUrl::Mainnet;

    let exchange_client = ExchangeClient::builder(url).build().await.unwrap();

    let modes: Vec<(String, bool)> = coins
        .iter()
        .map(|mode| (format!("{dex_name}:{mode}"), true))
        .collect();

    println!("Modes: {:?}", modes);
    let set_growth_modes = SetGrowthModes { modes };

    let signed_action = ActionKind::PerpDeploy(PerpDeploy::SetGrowthModes(set_growth_modes))
        .build(&exchange_client)
        .unwrap()
        .sign(&wallet)
        .unwrap();

    let set_growth_mode_result = exchange_client.send_action(signed_action).await;

    println!("Set growth mode result: {:?}", set_growth_mode_result);
}
