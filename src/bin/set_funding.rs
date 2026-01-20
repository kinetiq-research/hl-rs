use std::str::FromStr;

use alloy::signers::local::PrivateKeySigner;
use hl_rs::{
    exchange::{
        builder::BuildAction,
        requests::{PerpDeploy, SetFundingInterestRates},
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
    let dex_name = "dddd";
    let coins = vec!["MAK"];
    let exchange_client = ExchangeClient::builder(url).build().await.unwrap();

    let target_apy = 10.95;

    let periods = (365.0 * 24.0) / 8.0;
    let value = (target_apy * 0.01) / periods;

    for coin in coins {
        let set_funding_interest_rates = SetFundingInterestRates {
            rates: vec![(format!("{dex_name}:{coin}"), value.to_string())],
        };

        let signed_action = ActionKind::PerpDeploy(PerpDeploy::SetFundingInterestRates(
            set_funding_interest_rates,
        ))
        .build(&exchange_client)
        .unwrap()
        .sign(&wallet)
        .unwrap();
        let set_funding_interest_rates_result = exchange_client.send_action(signed_action).await;
        println!(
            "Set funding interest rates result for {coin}: {:?}",
            set_funding_interest_rates_result
        );
    }
}
