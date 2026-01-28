use alloy::primitives::Address;
use derive_builder::Builder;
use hl_rs_derive::{L1Action, UserSignedAction};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use super::{ser_lowercase, perp_deploy_v2::*};

// ============================================================================
// Example Action Implementations
// ============================================================================

/// Toggle big blocks mode for EVM user (L1 Action example)
#[derive(Debug, Clone, Serialize, Deserialize, Builder, L1Action)]
#[action(action_type = "evmUserModify")]
#[serde(rename_all = "camelCase")]
#[builder(setter(into))]
pub struct ToggleBigBlocks {
    pub using_big_blocks: bool,
    #[builder(default)]
    #[serde(skip_serializing)]
    pub nonce: Option<u64>,
}

impl ToggleBigBlocks {
    pub fn builder() -> ToggleBigBlocksBuilder {
        ToggleBigBlocksBuilder::default()
    }

    pub fn enable() -> Self {
        Self {
            using_big_blocks: true,
            nonce: None,
        }
    }

    pub fn disable() -> Self {
        Self {
            using_big_blocks: false,
            nonce: None,
        }
    }
}

/// USDC transfer between Hyperliquid accounts (UserSignedAction example)
#[derive(Debug, Clone, Serialize, Deserialize, Builder, UserSignedAction)]
#[action(types = "UsdSend(string hyperliquidChain,string destination,string amount,uint64 time)")]
#[serde(rename_all = "camelCase")]
#[builder(setter(into))]
pub struct UsdSend {
    #[serde(serialize_with = "ser_lowercase")]
    pub destination: Address,
    pub amount: Decimal,
    #[builder(default)]
    pub nonce: Option<u64>,
}

impl UsdSend {
    pub fn builder() -> UsdSendBuilder {
        UsdSendBuilder::default()
    }

    pub fn new(destination: Address, amount: Decimal) -> Self {
        Self {
            destination,
            amount,
            nonce: None,
        }
    }
}
macro_rules! impl_action_kind {
    ($($action:ident),*) => {
        #[derive(Debug, Clone)]
        pub enum ActionKind {
            $(
                $action($action),
            )*
        }
    };
}

// Use to convert SignedAction<T> to a concrete type.
impl_action_kind!(UsdSend, ToggleBigBlocks, SetOpenInterestCaps);
