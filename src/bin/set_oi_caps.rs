use std::str::FromStr;

use alloy::signers::local::PrivateKeySigner;
use hl_rs::{
    exchange::{
        builder::BuildAction,
        requests::{PerpDeploy, SetOpenInterestCaps},
        ActionKind,
    },
    BaseUrl, ExchangeClient,
};

#[tokio::main]
async fn main() {
    dotenv::dotenv().unwrap();

    let private_key = std::env::var("PRIVATE_KEY").unwrap();
    let wallet = PrivateKeySigner::from_str(&private_key).unwrap();

    let url = BaseUrl::Mainnet;
    let dex_name = "km";
    let collateral_units = 10_000_000;
    let caps = vec![("TSLA", collateral_units * 1_000_000)];

    let exchange_client = ExchangeClient::builder(url).build().await.unwrap();

    let set_oi_caps = SetOpenInterestCaps {
        caps: caps
            .iter()
            .map(|(coin, cap)| (format!("{dex_name}:{coin}"), *cap))
            .collect(),
    };
    let string =
        serde_json::to_string_pretty(&PerpDeploy::SetOpenInterestCaps(set_oi_caps.clone()))
            .unwrap();
    println!("String: {string}");

    let signed_action = ActionKind::PerpDeploy(PerpDeploy::SetOpenInterestCaps(set_oi_caps))
        .build(&exchange_client)
        .unwrap()
        .sign(&wallet)
        .unwrap();

    let set_oi_caps_result = exchange_client.send_action(signed_action).await;

    println!("Set OI caps result: {:?}", set_oi_caps_result);
}
