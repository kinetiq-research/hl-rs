use hl_rs_derive::L1Action;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, L1Action)]
#[action(action_type = "spotDeploy", payload_key = "enableFreezePrivilege")]
#[serde(rename_all = "camelCase")]
pub struct EnableFreezePrivilege {
    pub token: u32,
    #[serde(skip_serializing)]
    pub nonce: Option<u64>,
}

impl EnableFreezePrivilege {
    pub fn new(token: u32) -> Self {
        Self { token, nonce: None }
    }
}
