use std::{collections::HashMap, str::FromStr};

use alloy::signers::local::PrivateKeySigner;
use hl_rs::{
    exchange::{
        builder::BuildAction,
        requests::{PerpDeploy, SetMarginTableIds},
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
    let ids = vec![("TSLA", 12)]
        .iter()
        .map(|(mode, id)| (format!("{dex_name}:{mode}"), *id))
        .collect();

    let exchange_client = ExchangeClient::builder(url).build().await.unwrap();
    let set_margin_table_ids = SetMarginTableIds { ids };

    let signed_action = ActionKind::PerpDeploy(PerpDeploy::SetMarginTableIds(set_margin_table_ids))
        .build(&exchange_client)
        .unwrap()
        .sign(&wallet)
        .unwrap();

    let set_margin_table_ids_result = exchange_client.send_action(signed_action).await;

    println!(
        "Set margin table IDs result: {:?}",
        set_margin_table_ids_result
    );
}
