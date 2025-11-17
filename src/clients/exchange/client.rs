use std::collections::HashMap;

use alloy::primitives::Address;
use reqwest::Client;

use crate::{
    BaseUrl,
    exchange::{
        Action, ActionKind,
        builder::BuildAction,
        requests::{ApproveAgent, HaltTrading, PerpDeploy, PerpDexSchemaInput, UsdSend},
    },
    http::HttpClient,
    prelude::Result,
    utils::next_nonce,
};

#[derive(Debug, Clone)]
pub struct ExchangeClient {
    http_client: HttpClient,
    vault_address: Option<Address>,
    expires_after: Option<u64>,
    coin_to_asset: HashMap<String, u32>,
}

impl ExchangeClient {
    pub fn new(
        base_url: Option<BaseUrl>,
        vault_address: Option<Address>,
        expires_after: Option<u64>,
        coin_to_asset: HashMap<String, u32>,
    ) -> Result<Self> {
        let base_url = base_url.unwrap_or(BaseUrl::Mainnet);

        Ok(Self {
            http_client: HttpClient {
                client: Client::default(),
                base_url: base_url.get_url(),
            },
            vault_address,
            expires_after,
            coin_to_asset,
        })
    }

    pub(crate) fn vault_address(&self) -> Option<Address> {
        self.vault_address
    }

    pub(crate) fn is_mainnet(&self) -> bool {
        self.http_client.is_mainnet()
    }

    pub(crate) fn hyperliquid_chain(&self) -> String {
        if self.is_mainnet() {
            "Mainnet".to_string()
        } else {
            "Testnet".to_string()
        }
    }

    pub(crate) fn expires_after(&self) -> Option<u64> {
        self.expires_after
    }

    pub(crate) fn http_client(&self) -> &HttpClient {
        &self.http_client
    }

    pub fn coin_to_asset(&self) -> &HashMap<String, u32> {
        &self.coin_to_asset
    }

    pub fn approve_agent_action<T: Into<String>>(
        &self,
        agent_address: Address,
        agent_name: T,
    ) -> Result<Action> {
        let approve_agent = ApproveAgent {
            signature_chain_id: 421614,
            hyperliquid_chain: self.hyperliquid_chain(),
            agent_address,
            agent_name: Some(agent_name.into()),
            nonce: next_nonce() as u64,
        };

        ActionKind::ApproveAgent(approve_agent).build(self)
    }

    pub fn usdc_transfer_action<T: Into<String>>(
        &self,
        amount: T,
        destination: Address,
    ) -> Result<Action> {
        let usd_send = UsdSend {
            signature_chain_id: 421614,
            hyperliquid_chain: self.hyperliquid_chain(),
            destination: destination.to_string(),
            amount: amount.into(),
            time: next_nonce() as u64,
        };

        ActionKind::UsdSend(usd_send).build(self)
    }

    pub fn register_asset_action<T: Into<String>>(
        &self,
        max_gas: Option<i64>,
        coin: T,
        sz_decimals: u64,
        oracle_px: f64,
        margin_table_id: u64,
        only_isolated: bool,
        dex: T,
        schema: Option<PerpDexSchemaInput>,
    ) -> Result<Action> {
        use crate::exchange::{
            ActionKind,
            requests::{PerpDeploy, RegisterAsset, RegisterAssetRequest},
        };

        let schema = match schema {
            Some(schema) => Some(PerpDexSchemaInput {
                full_name: schema.full_name,
                collateral_token: schema.collateral_token,
                oracle_updater: schema.oracle_updater.map(|updater| updater.to_lowercase()),
            }),
            None => None,
        };

        let register_asset_request = RegisterAssetRequest {
            coin: coin.into(),
            sz_decimals,
            oracle_px: oracle_px.to_string(),
            margin_table_id,
            only_isolated,
        };

        let register_asset = RegisterAsset {
            max_gas,
            asset_request: register_asset_request,
            dex: dex.into(),
            schema,
        };

        ActionKind::PerpDeploy(PerpDeploy::RegisterAsset(register_asset)).build(self)
    }

    pub fn halt_trading_action<T: Into<String>>(&self, coin: T, is_halted: bool) -> Result<Action> {
        let perp_deploy = ActionKind::PerpDeploy(PerpDeploy::HaltTrading(HaltTrading {
            coin: coin.into(),
            is_halted,
        }));

        perp_deploy.build(self)
    }

    pub fn set_oracle_action(
        &self,
        dex: impl Into<String>,
        oracle_pxs: Vec<(String, String)>,
        mark_pxs: Vec<Vec<(String, String)>>,
        external_perp_pxs: Vec<(String, String)>,
    ) -> Result<Action> {
        use crate::exchange::{
            ActionKind,
            requests::{PerpDeploy, SetOracle},
        };

        let set_oracle = SetOracle {
            dex: dex.into(),
            oracle_pxs,
            mark_pxs,
            external_perp_pxs,
        };

        ActionKind::PerpDeploy(PerpDeploy::SetOracle(set_oracle)).build(self)
    }
}
