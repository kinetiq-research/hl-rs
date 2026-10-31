use hl_rs_derive::L1Action;
use serde::{Deserialize, Serialize};

use crate::actions::serialization::ser_lowercase_str;

/// Request linking a Core spot token to an ERC20 on HyperEVM (nested under `spotDeploy`).
///
/// See [HyperCore ↔ HyperEVM transfers](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/hyperevm/hypercore-less-than-greater-than-hyperevm-transfers).
#[derive(Serialize, Deserialize, Debug, Clone, L1Action)]
#[action(action_type = "spotDeploy", payload_key = "requestEvmContract")]
#[serde(rename_all = "camelCase")]
pub struct RequestEvmContract {
    pub token: u32,
    /// ERC20 contract address on the EVM (hex with `0x` prefix, lowercased when serialized).
    #[serde(serialize_with = "ser_lowercase_str")]
    pub address: String,
    /// Difference in wei decimals between Core and EVM (typically in `[-2, 18]`).
    pub evm_extra_wei_decimals: i32,
    #[serde(skip_serializing)]
    pub nonce: Option<u64>,
}

impl RequestEvmContract {
    pub fn new(token: u32, address: impl Into<String>, evm_extra_wei_decimals: i32) -> Self {
        Self {
            token,
            address: address.into(),
            evm_extra_wei_decimals,
            nonce: None,
        }
    }
}
