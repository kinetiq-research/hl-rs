use hl_rs_derive::L1Action;
use serde::{Deserialize, Serialize};

/// A single cancel request by client order ID.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CancelByCloidWire {
    /// Asset index
    pub asset: u32,
    /// Client order ID
    pub cloid: String,
}

/// Batch cancel orders by client order ID action.
///
/// Flattened like Python: `{ "type": "cancelByCloid", "cancels": [ ... ] }` (no double `cancels`).
#[derive(Serialize, Deserialize, Debug, Clone, L1Action)]
pub struct CancelByCloid {
    /// Cancel requests
    pub cancels: Vec<CancelByCloidWire>,
    #[serde(skip_serializing)]
    pub nonce: Option<u64>,
}

impl CancelByCloid {
    pub fn new(cancels: Vec<CancelByCloidWire>) -> Self {
        Self {
            cancels,
            nonce: None,
        }
    }

    pub fn single(asset: u32, cloid: impl Into<String>) -> Self {
        Self::new(vec![CancelByCloidWire {
            asset,
            cloid: cloid.into(),
        }])
    }
}
