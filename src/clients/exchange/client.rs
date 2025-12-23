use alloy::{primitives::Address, signers::local::PrivateKeySigner};
use alloy_signer::Signature;
use serde::{Deserialize, Serialize};

use crate::{
    exchange::{
        builder::BuildAction,
        client_builder::ExchangeClientBuilder,
        requests::{ApproveAgent, HaltTrading, PerpDeploy, PerpDexSchemaInput, UsdSend},
        types::{DexParams, RegisterAssetParams, SetOracleParams},
        Action, ActionKind, SignedAction,
    },
    http::HttpClient,
    prelude::Result,
    types::{CoinToAsset, Meta},
    utils::next_nonce,
    BaseUrl,
};

#[derive(Debug, Clone)]
pub struct ExchangeClient {
    pub(crate) http_client: HttpClient,
    pub(crate) signer_private_key: Option<PrivateKeySigner>,
    pub(crate) meta: Option<Meta>,
    pub(crate) vault_address: Option<Address>,
    pub(crate) expires_after: Option<u64>,
    pub(crate) coin_to_asset: CoinToAsset,
}

impl ExchangeClient {
    pub fn builder(base_url: BaseUrl) -> ExchangeClientBuilder {
        ExchangeClientBuilder::new(base_url)
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

    pub fn set_oracle(&self, oracle_params: SetOracleParams) -> Result<Action> {
        ActionKind::PerpDeploy(PerpDeploy::SetOracle(oracle_params.into())).build(self)
    }

    pub async fn send_action(
        &self,
        signed_action: SignedAction,
    ) -> Result<crate::exchange::responses::ExchangeResponse> {
        let SignedAction {
            action,
            signature,
            nonce,
            ..
        } = signed_action;
        let exchange_payload = ExchangePayload {
            action,
            signature,
            nonce,
            // vault_address: self.vault_address,
            // expires_after: self.expires_after,
        };

        let res = serde_json::to_string(&exchange_payload)
            .map_err(|e| crate::Error::JsonParse(e.to_string()))?;

        let output = self.http_client.post("/exchange", res).await?;

        let raw_response: crate::exchange::responses::ExchangeResponseStatusRaw =
            serde_json::from_str(&output).map_err(|e| crate::Error::JsonParse(e.to_string()))?;

        raw_response.into_result()
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
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExchangePayload {
    pub action: ActionKind,
    #[serde(serialize_with = "crate::exchange::action::serialize_sig")]
    pub signature: Signature,
    pub nonce: i64,
    // vault_address: Option<Address>,
    // expires_after: Option<i64>,
}
