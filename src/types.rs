use std::collections::HashMap;

use alloy::primitives::B128;
use serde::Deserialize;

use crate::{LOCAL_API_URL, MAINNET_API_URL, TESTNET_API_URL};

#[derive(Debug, Clone, Copy)]
pub enum BaseUrl {
    Localhost,
    Testnet,
    Mainnet,
}

impl BaseUrl {
    pub fn get_url(&self) -> String {
        match self {
            BaseUrl::Localhost => LOCAL_API_URL.to_string(),
            BaseUrl::Mainnet => MAINNET_API_URL.to_string(),
            BaseUrl::Testnet => TESTNET_API_URL.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CoinToAsset(HashMap<String, u32>);

impl CoinToAsset {
    pub fn new(mapping: HashMap<String, u32>) -> Self {
        Self(mapping)
    }

    pub fn as_map(&self) -> &HashMap<String, u32> {
        &self.0
    }

    pub fn into_map(self) -> HashMap<String, u32> {
        self.0
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct Meta {
    pub universe: Vec<AssetMeta>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SpotMeta {
    pub universe: Vec<SpotAssetMeta>,
    pub tokens: Vec<TokenInfo>,
}

impl SpotMeta {
    pub fn add_pair_and_name_to_index_map(
        &self,
        mut coin_to_asset: HashMap<String, u32>,
    ) -> HashMap<String, u32> {
        let index_to_name: HashMap<usize, &str> = self
            .tokens
            .iter()
            .map(|info| (info.index, info.name.as_str()))
            .collect();

        for asset in self.universe.iter() {
            let spot_ind: u32 = 10000 + asset.index as u32;
            let name_to_ind = (asset.name.clone(), spot_ind);

            let Some(token_1_name) = index_to_name.get(&asset.tokens[0]) else {
                continue;
            };

            let Some(token_2_name) = index_to_name.get(&asset.tokens[1]) else {
                continue;
            };

            coin_to_asset.insert(format!("{token_1_name}/{token_2_name}"), spot_ind);
            coin_to_asset.insert(name_to_ind.0, name_to_ind.1);
        }

        coin_to_asset
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum SpotMetaAndAssetCtxs {
    SpotMeta(SpotMeta),
    Context(Vec<SpotAssetContext>),
}

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum MetaAndAssetCtxs {
    Meta(Meta),
    Context(Vec<AssetContext>),
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SpotAssetContext {
    pub day_ntl_vlm: String,
    pub mark_px: String,
    pub mid_px: Option<String>,
    pub prev_day_px: String,
    pub circulating_supply: String,
    pub coin: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AssetContext {
    pub day_ntl_vlm: String,
    pub funding: String,
    pub impact_pxs: Option<Vec<String>>,
    pub mark_px: String,
    pub mid_px: Option<String>,
    pub open_interest: String,
    pub oracle_px: String,
    pub premium: Option<String>,
    pub prev_day_px: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AssetMeta {
    pub name: String,
    pub sz_decimals: u32,
    pub max_leverage: usize,
    #[serde(default)]
    pub only_isolated: Option<bool>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SpotAssetMeta {
    pub tokens: [usize; 2],
    pub name: String,
    pub index: usize,
    pub is_canonical: bool,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TokenInfo {
    pub name: String,
    pub sz_decimals: u8,
    pub wei_decimals: u8,
    pub index: usize,
    pub token_id: B128,
    pub is_canonical: bool,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PerpDex {
    pub name: String,
    pub full_name: String,
    pub deployer: String,
    pub deployer_fee_scale: String,
    pub fee_recipient: Option<String>,
    pub oracle_updater: Option<String>,
    pub asset_to_streaming_oi_cap: Vec<(String, String)>,
    pub last_deployer_fee_scale_change_time: String,
    // pub sub_deployers: Vec<Vec<String>>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PerpDexStatus {
    pub total_net_deposit: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserStakingSummary {
    pub delegated: String,
    pub undelegated: String,
    pub total_pending_withdrawal: String,
    pub n_pending_withdrawals: u64,
}
