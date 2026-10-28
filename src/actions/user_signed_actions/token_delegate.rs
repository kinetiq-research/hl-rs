use alloy::primitives::Address;
use derive_builder::Builder;
use hl_rs_derive::UserSignedAction;
use serde::{Deserialize, Serialize};

use crate::actions::serialization::ser_lowercase;

/// Delegate or undelegate tokens to a validator.
#[derive(Debug, Clone, Serialize, Deserialize, Builder, UserSignedAction)]
#[action(
    types = "TokenDelegate(string hyperliquidChain,address validator,uint64 wei,bool isUndelegate,uint64 nonce)"
)]
#[serde(rename_all = "camelCase")]
#[builder(setter(into))]
pub struct TokenDelegate {
    /// Validator address to delegate to
    #[serde(serialize_with = "ser_lowercase")]
    pub validator: Address,
    /// Amount in wei to delegate/undelegate
    pub wei: u64,
    /// True to undelegate, false to delegate
    pub is_undelegate: bool,
    #[builder(default)]
    pub nonce: Option<u64>,
}

impl TokenDelegate {
    pub fn builder() -> TokenDelegateBuilder {
        TokenDelegateBuilder::default()
    }

    /// Delegate tokens to a validator
    pub fn delegate(validator: Address, wei: u64) -> Self {
        Self {
            validator,
            wei,
            is_undelegate: false,
            nonce: None,
        }
    }

    /// Undelegate tokens from a validator
    pub fn undelegate(validator: Address, wei: u64) -> Self {
        Self {
            validator,
            wei,
            is_undelegate: true,
            nonce: None,
        }
    }
}
