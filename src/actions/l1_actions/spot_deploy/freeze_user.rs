use hl_rs_derive::L1Action;
use serde::{Deserialize, Serialize};

use crate::actions::serialization::ser_lowercase_str;

#[derive(Serialize, Deserialize, Debug, Clone, L1Action)]
#[action(action_type = "spotDeploy", payload_key = "freezeUser")]
#[serde(rename_all = "camelCase")]
pub struct FreezeUser {
    pub token: u32,
    /// User address (hex, lowercased when serialized).
    #[serde(serialize_with = "ser_lowercase_str")]
    pub user: String,
    pub freeze: bool,
    #[serde(skip_serializing)]
    pub nonce: Option<u64>,
}

impl FreezeUser {
    pub fn new(token: u32, user: impl Into<String>, freeze: bool) -> Self {
        Self {
            token,
            user: user.into(),
            freeze,
            nonce: None,
        }
    }
}
