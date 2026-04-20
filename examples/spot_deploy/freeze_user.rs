use std::str::FromStr;

use alloy::signers::local::PrivateKeySigner;
use hl_rs::{BaseUrl, ExchangeClient, FreezeUser};

#[tokio::main]
async fn main() {
    dotenv::dotenv().unwrap();

    let token: u32 = std::env::var("SPOT_TOKEN")
        .ok()
        .and_then(|s| s.parse().ok())
        .expect("SPOT_TOKEN");

    let target = std::env::var("FREEZE_USER").expect("FREEZE_USER (lowercase 0x address)");
    let freeze: bool = std::env::var("FREEZE")
        .map(|s| s == "1" || s.eq_ignore_ascii_case("true"))
        .unwrap_or(true);

    let action = FreezeUser::new(token, target.to_lowercase(), freeze);

    let private_key = std::env::var("PRIVATE_KEY").unwrap();
    let wallet = PrivateKeySigner::from_str(&private_key).unwrap();
    println!("wallet: {}", wallet.address());

    let client = ExchangeClient::new(BaseUrl::Testnet).with_signer(wallet);
    let result = client.send_action(action).await.unwrap();
    println!("freeze_user: {:?}", result);
}
