use hl_rs_derive::L1Action;
use serde::{Deserialize, Serialize};

use super::OrderWire;

/// A single modify request.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModifyWire {
    /// Order ID to modify
    pub oid: u64,
    /// New order details
    pub order: OrderWire,
}

/// Batch modify orders action.
///
/// JSON and msgpack signing must match the official Python SDK:
/// `{ "type": "batchModify", "modifies": [ { "oid", "order" }, ... ] }`.
///
/// `PAYLOAD_KEY` must **not** be `"modifies"`: the wire struct already has a `modifies` field. Using
/// a different payload key nests the payload as `"modifies": { "modifies": [...] }`, which Hyperliquid
/// rejects with HTTP 422 (`Failed to deserialize the JSON body into the target type`).
#[derive(Serialize, Deserialize, Debug, Clone, L1Action)]
pub struct BatchModify {
    /// Modify requests
    pub modifies: Vec<ModifyWire>,
    #[serde(skip_serializing)]
    pub nonce: Option<u64>,
}

impl BatchModify {
    pub fn new(modifies: Vec<ModifyWire>) -> Self {
        Self {
            modifies,
            nonce: None,
        }
    }

    pub fn single(oid: u64, order: OrderWire) -> Self {
        Self::new(vec![ModifyWire { oid, order }])
    }
}
