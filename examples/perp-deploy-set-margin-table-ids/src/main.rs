use std::str::FromStr;

use alloy::signers::local::PrivateKeySigner;
use hl_rs::{
    exchange::{
        requests::{PerpDeploy, SetMarginTableIds},
        ActionKind,
    },
    BaseUrl, ExchangeClient,

};

use hl_rs::exchange::builder::BuildAction;

#[tokio::main]
async fn main() {
    dotenv::dotenv().unwrap();

    let agent_private_key = std::env::var("AGENT_PRIVATE_KEY").unwrap();
    let agent_wallet = PrivateKeySigner::from_str(&agent_private_key).unwrap();

    let exchange_client = ExchangeClient::builder(BaseUrl::Testnet)
        .build()
        .await
        .unwrap();

    let set_margin_table_ids = SetMarginTableIds {
        ids: vec![
            ("slob:TEST0".to_string(), 1),
            ("slob:TEST1".to_string(), 2),
            ("slob:TEST2".to_string(), 49),
            // Add more asset and margin table ID pairs as needed
        ],
    };

    let signed_action = ActionKind::PerpDeploy(PerpDeploy::SetMarginTableIds(set_margin_table_ids))
        .build(&exchange_client)
        .unwrap()
        .sign(&agent_wallet)
        .unwrap();

    let payload = serde_json::to_string(&signed_action.action).unwrap();

    let set_margin_table_ids_result = exchange_client
        .send_action(signed_action)
        .await;

    println!("Set margin table IDs result: {:?}", set_margin_table_ids_result);
}

