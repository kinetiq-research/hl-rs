use alloy::{
    primitives::{Address, Signature},
    signers::local::PrivateKeySigner,
};
use serde::{ser::SerializeStruct, Deserialize, Serialize, Serializer};

use crate::{
    exchange::{ActionKind, ExchangeClient},
    utils::{recover_action, SigningData},
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
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Action {
    pub action: ActionKind,
    pub nonce: u64,
    pub vault_address: Option<Address>,
    pub expires_after: Option<u64>,
    pub signing_data: SigningData,
}

/// Signed action ready to be sent to the Hyperliquid API.
///
/// This action has been fully prepared and signed, and can be sent
/// immediately to the exchange.
#[derive(Debug, Deserialize, Serialize)]
pub struct SignedAction {
    pub action: ActionKind,
    pub nonce: u64,
    pub signature: Signature,
    pub vault_address: Option<Address>,
    pub expires_after: Option<u64>,
}

impl Action {
    /// Sign action with the provided wallet.
    pub fn sign(self, wallet: &PrivateKeySigner) -> Result<SignedAction, Error> {
        Ok(SignedAction {
            action: self.action,
            nonce: self.nonce,
            signature: self.signing_data.sign(wallet)?,
            vault_address: self.vault_address,
            expires_after: self.expires_after,
        })
    }

    /// Attach externally-provided signature to this action.
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
        recover_action(exchange_client.base_url.get_signing_chain(), &self)
    }
}
