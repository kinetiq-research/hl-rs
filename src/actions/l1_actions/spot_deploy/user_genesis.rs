use hl_rs_derive::L1Action;
use serde::{Deserialize, Serialize};

use crate::actions::serialization::ser_lowercase_users;

/// User genesis allocation for token distribution.
#[derive(Serialize, Deserialize, Debug, Clone, L1Action)]
#[action(action_type = "spotDeploy", payload_key = "userGenesis")]
#[serde(rename_all = "camelCase")]
pub struct UserGenesis {
    /// Token index
    pub token: u32,
    /// User addresses and their wei allocations (addresses lowercased when serialized)
    #[serde(serialize_with = "ser_lowercase_users")]
    pub user_and_wei: Vec<(String, String)>,
    /// Existing token IDs and their wei allocations
    pub existing_token_and_wei: Vec<(u32, String)>,
    #[serde(skip_serializing)]
    pub nonce: Option<u64>,
}

impl UserGenesis {
    pub fn new(
        token: u32,
        user_and_wei: Vec<(String, String)>,
        existing_token_and_wei: Vec<(u32, String)>,
    ) -> Self {
        Self {
            token,
            user_and_wei,
            existing_token_and_wei,
            nonce: None,
        }
    }
}
