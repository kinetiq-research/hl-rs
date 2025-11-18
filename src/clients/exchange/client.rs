use std::collections::HashMap;

use alloy::primitives::Address;
use reqwest::Client;

use crate::{
    exchange::{
        builder::BuildAction,
        requests::{ApproveAgent, HaltTrading, PerpDeploy, PerpDexSchemaInput, UsdSend},
        types::{DexParams, RegisterAssetParams},
        Action, ActionKind,
    },
    http::HttpClient,
    prelude::Result,
    utils::next_nonce,
    BaseUrl,
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

    pub fn register_asset_on_new_dex(
        &self,
        dex_params: DexParams,
        asset_params: RegisterAssetParams,
    ) -> Result<Action> {
        let dex_name = dex_params.full_name.to_owned();
        self.register_asset(
            asset_params.max_gas,
            asset_params.ticker.as_str(),
            asset_params.size_decimals,
            asset_params.oracle_price,
            asset_params.margin_table_id,
            asset_params.only_isolated,
            dex_name.as_str(),
            Some(dex_params.into()),
        )
    }

    pub fn register_asset_on_existing_dex(
        &self,
        dex_name: &str,
        asset_params: RegisterAssetParams,
    ) -> Result<Action> {
        self.register_asset(
            asset_params.max_gas,
            asset_params.ticker.as_str(),
            asset_params.size_decimals,
            asset_params.oracle_price,
            asset_params.margin_table_id,
            asset_params.only_isolated,
            dex_name,
            None,
        )
    }

    fn register_asset(
        &self,
        max_gas: Option<u64>,
        coin: &str,
        sz_decimals: u64,
        oracle_px: f64,
        margin_table_id: u64,
        only_isolated: bool,
        dex: &str,
        schema: Option<PerpDexSchemaInput>,
    ) -> Result<Action> {
        use crate::exchange::{
            requests::{PerpDeploy, RegisterAsset, RegisterAssetRequest},
            ActionKind,
        };

        let register_asset_request = RegisterAssetRequest {
            coin: coin.to_string(),
            sz_decimals,
            oracle_px: oracle_px.to_string(),
            margin_table_id,
            only_isolated,
        };

        let register_asset = RegisterAsset {
            max_gas,
            asset_request: register_asset_request,
            dex: dex.to_string(),
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
            requests::{PerpDeploy, SetOracle},
            ActionKind,
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
