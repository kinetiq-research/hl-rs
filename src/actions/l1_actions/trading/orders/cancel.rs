use hl_rs_derive::L1Action;
use serde::{Deserialize, Serialize};

/// A single cancel request by order ID.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CancelWire {
    /// Asset index
    pub a: u32,
    /// Order ID to cancel
    pub o: u64,
}

/// Batch cancel orders action.
///
/// Wire shape must match Python: `{ "type": "cancel", "cancels": [ { "a", "o" }, ... ] }`.
/// - `payload_key = "cancels"` incorrectly nests as `"cancels": { "cancels": [...] }` → HTTP 422.
/// - Default derive uses `type: "batchCancel"`; override with `action_type = "cancel"`.
#[derive(Serialize, Deserialize, Debug, Clone, L1Action)]
#[action(action_type = "cancel")]
pub struct BatchCancel {
    /// Cancel requests
    pub cancels: Vec<CancelWire>,
    #[serde(skip_serializing)]
    pub nonce: Option<u64>,
}

impl BatchCancel {
    pub fn new(cancels: Vec<CancelWire>) -> Self {
        Self {
            cancels,
            nonce: None,
        }
    }

    pub fn single(asset: u32, oid: u64) -> Self {
        Self::new(vec![CancelWire { a: asset, o: oid }])
    }
}
