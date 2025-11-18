use crate::exchange::requests::{PerpDexSchemaInput, SetOracle};

#[derive(Debug, Clone)]
pub struct RegisterAssetParams {
    pub max_gas: Option<u64>,
    pub ticker: String,
    pub size_decimals: u64,
    pub oracle_price: f64,
    pub margin_table_id: u64,
    pub only_isolated: bool,
}

#[derive(Debug, Clone)]
pub struct DexParams {
    pub full_name: String,
    pub collateral_token: u64,
    pub oracle_updater: Option<String>,
}

impl From<DexParams> for PerpDexSchemaInput {
    fn from(params: DexParams) -> Self {
        Self {
            full_name: params.full_name,
            collateral_token: params.collateral_token,
            oracle_updater: params.oracle_updater.map(|s| s.to_lowercase()),
        }
    }
}

// The mark_prices outer list can be length 0, 1, or 2. The median of these inputs
// along with the local mark price (median(best bid, best ask, last trade price))
// is used as the new mark price update.
//
// SetOracle can be called multiple times but there must be at least 2.5 seconds between calls.
//
// Stale mark_prices will be updated to the local mark_price after 10 seconds of no updates.
// This fallback counts as an update for purposes of the maximal update frequency.
// This fallback behavior should not be relied upon. Deployers are expected to call setOracle every 3 seconds even with no changes.
//
// All prices are clamped to 10x the start of day value.
// mark_px moves are clamped to 1% from previous mark_px.
// mark_px cannot be updated such that open interest would be 10x the open interest cap.
//
// @param dex - Name of the perp dex (<= 6 characters)
// @param oracle_prices - A list (sorted by key) of asset and oracle prices.
// @param mark_prices - An outer list of inner lists (inner list sorted by key) of asset and mark prices.
// @param external_perp_prices - A list (sorted by key) of asset and external prices which prevent sudden mark price deviations.
//                          Ideally externally determined by deployer, but could fall back to an EMA of recent mark prices.
//                          Must include all assets.
pub struct SetOracleParams {
    pub dex_name: String,
    pub oracle_prices: Vec<OraclePrice>,
    pub mark_prices: Vec<Vec<MarkPrice>>,
    pub external_perp_prices: Vec<ExternalPerpPrice>,
}

pub struct OraclePrice {
    pub asset: String,
    pub price: String,
}

pub struct MarkPrice {
    pub asset: String,
    pub price: String,
}

pub struct ExternalPerpPrice {
    pub asset: String,
    pub price: String,
}

impl From<SetOracleParams> for SetOracle {
    fn from(params: SetOracleParams) -> Self {
        Self {
            dex: params.dex_name,
            oracle_pxs: params
                .oracle_prices
                .into_iter()
                .map(|oracle| (oracle.asset, oracle.price))
                .collect(),
            mark_pxs: params
                .mark_prices
                .into_iter()
                .map(|mark| mark.into_iter().map(|m| (m.asset, m.price)).collect())
                .collect(),
            external_perp_pxs: params
                .external_perp_prices
                .into_iter()
                .map(|external_perp| (external_perp.asset, external_perp.price))
                .collect(),
        }
    }
}
