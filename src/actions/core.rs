// ============================================================================
// Prepared and Signed Action Types
// ============================================================================

use alloy::{
    primitives::{Address, B256},
    signers::local::PrivateKeySigner,
};
use alloy_signer::{Signature, SignerSync};

use crate::{
    actions::{signing::SigningMeta, traits::Action, ActionKind},
    Error, SigningChain,
};
use serde_json;

/// Action prepared with metadata, ready for signing
pub struct PreparedAction<A: Action> {
    pub action: A,
    pub nonce: u64,
    pub vault_address: Option<Address>,
    pub expires_after: Option<u64>,
    signing_chain: SigningChain,
    signing_hash: B256,
}

impl<A: Action> PreparedAction<A> {
    /// Prepare an action for signing
    pub fn new(
        action: A,
        signing_chain: &SigningChain,
        vault_address: Option<Address>,
        expires_after: Option<u64>,
    ) -> Result<Self, Error> {
        let nonce = action.nonce().unwrap_or_else(current_timestamp_ms);
        let action = action.with_nonce(nonce);

        let meta = SigningMeta {
            nonce,
            vault_address,
            expires_after,
            signing_chain,
        };

        let signing_hash = action.signing_hash(&meta)?;

        Ok(PreparedAction {
            action,
            nonce,
            vault_address,
            expires_after,
            signing_chain: signing_chain.clone(),
            signing_hash,
        })
    }
    /// Get the hash that needs to be signed
    pub fn signing_hash(&self) -> B256 {
        self.signing_hash
    }

    /// Sign with a local wallet
    pub fn sign(self, wallet: &PrivateKeySigner) -> Result<SignedAction<A>, Error> {
        let signature = wallet
            .sign_hash_sync(&self.signing_hash)
            .map_err(|e| Error::SignatureFailure(e.to_string()))?;

        Ok(SignedAction {
            action: self.action,
            nonce: self.nonce,
            vault_address: self.vault_address,
            expires_after: self.expires_after,
            signature,
            signing_chain: Some(self.signing_chain),
        })
    }

    /// Attach an externally-provided signature
    pub fn with_signature(self, signature: Signature) -> SignedAction<A> {
        SignedAction {
            action: self.action,
            nonce: self.nonce,
            vault_address: self.vault_address,
            expires_after: self.expires_after,
            signature,
            signing_chain: Some(self.signing_chain),
        }
    }
}

/// Fully signed action ready to submit
///
/// Serializes to the exchange API format:
/// ```json
/// {
///   "action": { "type": "actionType", ...payload...  },
///   "nonce": 12345,
///   "signature": { "r": "0x...", "s": "0x...", "v": 27 },
///   "vaultAddress": "0x...",  // optional
///   "expiresAfter": 12345     // optional
/// }
/// ```
#[derive(Debug)]
pub struct SignedAction<T: Action> {
    pub action: T,
    pub nonce: u64,
    pub signature: Signature,
    pub vault_address: Option<Address>,
    pub expires_after: Option<u64>,
    pub signing_chain: Option<SigningChain>,
}

impl<T: Action> SignedAction<T> {
    /// Extract the action kind (clones the inner action into an ActionKind variant)
    pub fn extract_action_kind(&self) -> ActionKind {
        self.action.extract_action_kind()
    }
}

/// Fully signed action envelope, deserialized without a concrete action type.
///
/// This is primarily useful for reading signed actions from JSON when you only
/// have the runtime `"action.type"` tag, not the compile-time `T`.
#[derive(Debug)]
pub struct SignedActionKind {
    pub action: ActionKind,
    pub nonce: u64,
    pub signature: Signature,
    pub vault_address: Option<Address>,
    pub expires_after: Option<u64>,
    pub signing_chain: Option<SigningChain>,
}

impl SignedActionKind {
    /// Deserialize from the exchange API format without knowing the concrete action type.
    pub fn from_json(json: &str) -> Result<Self, Error> {
        serde_json::from_str(json).map_err(|e| Error::JsonParse(e.to_string()))
    }
}

/// Get current timestamp in milliseconds.
/// Returns 0 when the system clock is before the Unix epoch instead of
/// panicking, so clock skew can never crash signing paths.
pub fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Fallible timestamp helper for callers that need an explicit error when
/// the system clock is unavailable or set before the Unix epoch.
pub fn try_current_timestamp_ms() -> Result<u64, crate::Error> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .map_err(|e| crate::Error::GenericRequest(e.to_string()))
}
