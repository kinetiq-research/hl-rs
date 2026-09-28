use std::str::FromStr;

use alloy::signers::local::PrivateKeySigner;
use hl_rs::{actions::DeployerFeeConfig, BaseUrl, ExchangeClient, PerpSetDeployerFees};

#[tokio::main]
async fn main() {
    dotenv::dotenv().unwrap();

    let url = BaseUrl::Mainnet;
    let dex_name = "mkts";

    // Set deployer fee scale and growth mode per asset.
    // On mainnet, changes are rate limited to one per 30 days per asset.
    let action = PerpSetDeployerFees::new(
        dex_name,
        vec![(
            "USBOND",
            DeployerFeeConfig::new("1.0", true), // scale in [0.0, 3.0]
        )],
    );

    let private_key = std::env::var("PRIVATE_KEY").unwrap();
    let wallet = PrivateKeySigner::from_str(&private_key).unwrap();
    println!("wallet: {}", wallet.address());

    let client = ExchangeClient::new(url).with_signer(wallet);

    let result = client.send_action(action).await.unwrap();

    println!("Set deployer fees result: {:?}", result);
}
