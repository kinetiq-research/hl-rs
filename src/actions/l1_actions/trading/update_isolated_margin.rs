use hl_rs_derive::L1Action;
use serde::{Deserialize, Serialize};

/// Update isolated margin for a position.
#[derive(Serialize, Deserialize, Debug, Clone, L1Action)]
#[serde(rename_all = "camelCase")]
pub struct UpdateIsolatedMargin {
    /// Asset index
    pub asset: u32,
    /// True for buy side, false for sell side
    pub is_buy: bool,
    /// Net transfer to isolated margin (positive to add, negative to remove)
    pub ntli: i64,
    #[serde(skip_serializing)]
    pub nonce: Option<u64>,
}

impl UpdateIsolatedMargin {
    pub fn add(asset: u32, is_buy: bool, amount: i64) -> Self {
        Self {
            asset,
            is_buy,
            ntli: amount.saturating_abs(),
            nonce: None,
        }
    }

    pub fn remove(asset: u32, is_buy: bool, amount: i64) -> Self {
        Self {
            asset,
            is_buy,
            ntli: amount.saturating_abs().saturating_neg(),
            nonce: None,
        }
    }
}
