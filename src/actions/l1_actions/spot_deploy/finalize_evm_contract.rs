use hl_rs_derive::L1Action;
use serde::{Deserialize, Serialize};

/// Finalize linking after [`RequestEvmContract`] (top-level L1 action, not nested under `spotDeploy`).
#[derive(Serialize, Deserialize, Debug, Clone, L1Action)]
#[action(action_type = "finalizeEvmContract")]
#[serde(rename_all = "camelCase")]
pub struct FinalizeEvmContract {
    pub token: u32,
    pub input: FinalizeEvmContractInput,
    #[serde(skip_serializing)]
    pub nonce: Option<u64>,
}

/// `input` for [`FinalizeEvmContract`]: either EOA deploy flow or storage-slot finalizers.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum FinalizeEvmContractInput {
    ByCreate(FinalizeByCreate),
    /// `"firstStorageSlot"` or `"customStorageSlot"`.
    Slot(String),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FinalizeByCreate {
    pub create: CreateNoncePayload,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateNoncePayload {
    pub nonce: u64,
}

impl FinalizeEvmContract {
    pub fn by_create_nonce(token: u32, deploy_tx_nonce: u64) -> Self {
        Self {
            token,
            input: FinalizeEvmContractInput::ByCreate(FinalizeByCreate {
                create: CreateNoncePayload {
                    nonce: deploy_tx_nonce,
                },
            }),
            nonce: None,
        }
    }

    pub fn first_storage_slot(token: u32) -> Self {
        Self {
            token,
            input: FinalizeEvmContractInput::Slot("firstStorageSlot".to_string()),
            nonce: None,
        }
    }

    pub fn custom_storage_slot(token: u32) -> Self {
        Self {
            token,
            input: FinalizeEvmContractInput::Slot("customStorageSlot".to_string()),
            nonce: None,
        }
    }
}
