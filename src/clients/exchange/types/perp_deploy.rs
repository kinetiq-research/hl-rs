use crate::exchange::requests::PerpDexSchemaInput;

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
