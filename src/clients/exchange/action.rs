use alloy::{
    primitives::{Address, Signature, B256},
    signers::{local::PrivateKeySigner, SignerSync},
};
use serde::{ser::SerializeStruct, Deserialize, Serialize, Serializer};

use crate::{
    exchange::{self, ActionKind, ExchangeClient},
    utils::{recover_action, sign_l1_action},
    Error,
};

pub fn serialize_sig<S>(sig: &Signature, s: S) -> std::result::Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut state = s.serialize_struct("Signature", 3)?;
    state.serialize_field("r", &sig.r())?;
    state.serialize_field("s", &sig.s())?;
    state.serialize_field("v", &(27 + sig.v() as u64))?;
    state.end()
}

/// Unsigned action ready to be signed.
///
/// Represents a fully prepared action that has been built with all
/// necessary metadata (timestamp, vault address, signing data) but has
/// not yet been signed.
///
#[derive(Debug, Serialize, Deserialize)]
pub struct Action {
    pub action: ActionKind,
    pub nonce: i64,
    pub vault_address: Option<Address>,
    pub expires_after: Option<i64>,
    pub signing_data: SigningData,
}

/// Enum representing data needed for signing an action.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum SigningData {
    /// L1 actions require a connection_id and network type.
    L1 {
        connection_id: B256,
        is_mainnet: bool,
    },
    /// Typed data actions require only the EIP-712 hash.
    TypedData { hash: B256 },
}

/// Signed action ready to be sent to the Hyperliquid API.
///
/// This action has been fully prepared and signed, and can be sent
/// immediately to the exchange.
#[derive(Debug, Deserialize, Serialize)]
pub struct SignedAction {
    pub action: ActionKind,
    pub nonce: i64,
    pub signature: Signature,
    pub vault_address: Option<Address>,
    pub expires_after: Option<i64>,
}

impl Action {
    /// Sign action with the provided wallet.
    pub fn sign(self, wallet: &PrivateKeySigner) -> Result<SignedAction, Error> {
        let signature = match self.signing_data {
            SigningData::L1 {
                connection_id,
                is_mainnet,
            } => sign_l1_action(wallet, connection_id, is_mainnet)?,
            SigningData::TypedData { hash } => wallet
                .sign_hash_sync(&hash)
                .map_err(|e| Error::SignatureFailure(e.to_string()))?,
        };

        Ok(SignedAction {
            action: self.action,
            nonce: self.nonce,
            signature,
            vault_address: self.vault_address,
            expires_after: self.expires_after,
        })
    }

    /// Attach externally-provided signature to this action.
    /// Use this when signing is done outside the SDK (e.g., using Nitro Enclave).
    pub fn with_signature(self, signature: Signature) -> SignedAction {
        SignedAction {
            action: self.action,
            nonce: self.nonce,
            signature,
            vault_address: self.vault_address,
            expires_after: self.expires_after,
        }
    }

    /// Get signing data needed for external signing.
    pub fn signing_data(&self) -> &SigningData {
        &self.signing_data
    }
}

impl SignedAction {
    pub fn recover_user(&self, exchange_client: &ExchangeClient) -> Result<Address, Error> {
        recover_action(exchange_client, &self)
    }
}
