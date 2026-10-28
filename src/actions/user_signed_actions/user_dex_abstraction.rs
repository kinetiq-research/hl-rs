use alloy::primitives::Address;
use derive_builder::Builder;
use hl_rs_derive::UserSignedAction;
use serde::{Deserialize, Serialize};

use crate::actions::serialization::ser_lowercase;

/// Enable or disable DEX abstraction for a user.
#[derive(Debug, Clone, Serialize, Deserialize, Builder, UserSignedAction)]
#[action(
    types = "UserDexAbstraction(string hyperliquidChain,address user,bool enabled,uint64 nonce)"
)]
#[serde(rename_all = "camelCase")]
#[builder(setter(into))]
pub struct UserDexAbstraction {
    /// User address to modify
    #[serde(serialize_with = "ser_lowercase")]
    pub user: Address,
    /// Whether DEX abstraction is enabled
    pub enabled: bool,
    #[builder(default)]
    pub nonce: Option<u64>,
}

impl UserDexAbstraction {
    pub fn builder() -> UserDexAbstractionBuilder {
        UserDexAbstractionBuilder::default()
    }

    pub fn enable(user: Address) -> Self {
        Self {
            user,
            enabled: true,
            nonce: None,
        }
    }

    pub fn disable(user: Address) -> Self {
        Self {
            user,
            enabled: false,
            nonce: None,
        }
    }
}
