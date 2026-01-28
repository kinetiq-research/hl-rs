//! New action type system with strongly typed generics.
//!
//! This module provides a trait-based action system where:
//! - `L1Action` trait: for actions signed via connection_id mechanism
//! - `UserSignedAction` trait: for actions signed via EIP-712 typed data
//! - `Action` trait: unified interface auto-implemented for both
//! - `SignedAction<T>`: strongly typed signed action ready to submit

use alloy::{
    dyn_abi::Eip712Domain,
    primitives::{keccak256, Address, Signature, B256, U256},
    signers::{local::PrivateKeySigner, SignerSync},
    sol,
    sol_types::{eip712_domain, SolStruct},
};
use reqwest::Client;
use serde::{
    de::DeserializeOwned,
    ser::{Error as SerError, SerializeMap, SerializeStruct},
    Deserialize, Deserializer, Serialize, Serializer,
};

use crate::{http::HttpClient, BaseUrl, Error, SigningChain};

mod action_v2_impls;
pub use action_v2_impls::*;
mod perp_deploy_v2;
pub use perp_deploy_v2::*;

// ============================================================================
// Core Types
// ============================================================================

/// Metadata passed during action signing
#[derive(Debug, Clone)]
pub struct SigningMeta<'a> {
    pub nonce: u64,
    pub vault_address: Option<Address>,
    pub expires_after: Option<u64>,
    pub signing_chain: &'a SigningChain,
}

// EIP-712 Agent struct for L1 action signing
sol! {
    #[derive(Debug)]
    struct Agent {
        string source;
        bytes32 connectionId;
    }
}

fn agent_domain() -> Eip712Domain {
    eip712_domain! {
        name: "Exchange",
        version: "1",
        chain_id: 1337,
        verifying_contract: Address::ZERO,
    }
}

fn agent_signing_hash(connection_id: B256, source: &str) -> B256 {
    let agent = Agent {
        source: source.to_string(),
        connectionId: connection_id,
    };

    let domain_hash = agent_domain().hash_struct();
    let struct_hash = agent.eip712_hash_struct();

    let mut digest = [0u8; 66];
    digest[0] = 0x19;
    digest[1] = 0x01;
    digest[2..34].copy_from_slice(&domain_hash[..]);
    digest[34..66].copy_from_slice(&struct_hash[..]);

    keccak256(&digest)
}

// ============================================================================
// L1Action Trait
// ============================================================================

/// Trait for actions signed via L1 connection_id mechanism.
///
/// L1 actions are hashed using MessagePack serialization combined with
/// timestamp, vault address, and expiration metadata.
pub trait L1Action: Serialize + Clone + Send + Sync + 'static {
    /// Action type name for API serialization (e.g., "order", "updateLeverage")
    const ACTION_TYPE: &'static str;

    /// Key to serialize action payload to.
    const PAYLOAD_KEY: &'static str;

    /// Whether to exclude vault_address from the hash (default: false)
    /// Override to `true` for actions like PerpDeploy
    const EXCLUDE_VAULT_FROM_HASH: bool = false;
}

/// Compute the L1 action hash (MessagePack + metadata)
pub(crate) struct L1ActionWrapper<'a, T: Action + Serialize> {
    pub action: &'a T,
}

impl<'a, T: Action + Serialize> Serialize for L1ActionWrapper<'a, T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let payload = serde_json::to_value(self.action).map_err(S::Error::custom)?;

        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("type", T::action_type())?;
        if T::payload_key() == T::action_type() {
            let payload = payload
                .as_object()
                .cloned()
                .ok_or_else(|| S::Error::custom("action payload must be object"))?;
            for (key, value) in payload {
                map.serialize_entry(&key, &value)?;
            }
        } else {
            map.serialize_entry(T::payload_key(), &payload)?;
        }
        map.end()
    }
}

fn compute_l1_hash<T: Serialize>(
    action: &T,
    nonce: u64,
    vault_address: Option<Address>,
    expires_after: Option<u64>,
) -> Result<B256, Error> {
    let mut bytes = rmp_serde::to_vec_named(action).map_err(|e| Error::RmpParse(e.to_string()))?;

    bytes.extend(nonce.to_be_bytes());

    match vault_address {
        Some(addr) => {
            bytes.push(1);
            bytes.extend(addr.as_slice());
        }
        None => bytes.push(0),
    }

    if let Some(expires) = expires_after {
        bytes.push(0);
        bytes.extend(expires.to_be_bytes());
    }

    Ok(keccak256(&bytes))
}

// ============================================================================
// UserSignedAction Trait
// ============================================================================

/// Trait for actions signed via EIP-712 typed data.
///
/// User-signed actions produce human-readable signature requests in wallets.
/// They embed their own timestamp/nonce and use the Hyperliquid typed data domain.
///
/// # Examples
/// - UsdSend, Withdraw3, SpotSend
/// - ApproveAgent, ApproveBuilderFee
pub trait UserSignedAction: Serialize + Send + Sync + 'static {
    /// Action type name for API serialization (e.g., "usdSend", "withdraw3")
    const ACTION_TYPE: &'static str;

    /// Compute the EIP-712 struct hash (type-specific)
    fn struct_hash(&self, chain: &SigningChain) -> B256;

    /// Compute the full EIP-712 signing hash
    fn eip712_signing_hash(&self, chain: &SigningChain) -> B256 {
        let domain = eip712_domain! {
            name: "HyperliquidSignTransaction",
            version: "1",
            chain_id: chain.signature_chain_id(),
            verifying_contract: Address::ZERO,
        };

        let domain_hash = domain.hash_struct();
        let struct_hash = self.struct_hash(chain);

        let mut digest = [0u8; 66];
        digest[0] = 0x19;
        digest[1] = 0x01;
        digest[2..34].copy_from_slice(&domain_hash[..]);
        digest[34..66].copy_from_slice(&struct_hash[..]);

        keccak256(&digest)
    }
}

impl SigningChain {
    /// EIP-712 signature chain ID (Arbitrum)
    pub fn signature_chain_id(&self) -> u64 {
        match self {
            SigningChain::Mainnet => 42161,
            SigningChain::Testnet => 421614,
            #[cfg(feature = "custom-signing-chain")]
            SigningChain::Custom { .. } => 421614, // Add as param to Custom SigningChain
        }
    }
}

// ============================================================================
// Action Trait (Unified Interface)
// ============================================================================

/// Unified trait for all exchange actions.
///
/// This trait is implemented for action types via the provided macros.
/// Client code uses this trait for a uniform API.
pub trait Action: Serialize + Send + Sync {
    /// Action type name for 'type' tag serialization
    fn action_type() -> &'static str;

    /// Key to serialize action payload to.
    fn payload_key() -> &'static str {
        Self::action_type()
    }

    /// Whether the action is user-signed (EIP-712).
    fn is_user_signed() -> bool {
        false
    }

    /// Whether the user-signed payload uses `time` (vs `nonce`).
    fn uses_time() -> bool {
        false
    }

    /// Build the signing hash from action and metadata
    fn signing_hash(&self, meta: &SigningMeta) -> Result<B256, Error>;

    /// Build the multisig signing hash (matches Python SDK multi-sig payload rules)
    fn multisig_signing_hash(
        &self,
        meta: &SigningMeta,
        payload_multi_sig_user: Address,
        outer_signer: Address,
    ) -> Result<B256, Error>;

    /// Get embedded nonce from the action (if present)
    fn nonce(&self) -> Option<u64>;

    /// Extract the action kind (clones the action into an ActionKind variant)
    fn extract_action_kind(&self) -> ActionKind;

    /// Attach an embedded nonce to the action
    fn with_nonce(self, nonce: u64) -> Self;
}

// ============================================================================
// Prepared and Signed Action Types
// ============================================================================

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

fn build_action_value<T: Action + Serialize>(
    action: &T,
    signing_chain: Option<&SigningChain>,
) -> Result<serde_json::Value, String> {
    let payload_value = serde_json::to_value(action).map_err(|e| e.to_string())?;
    let mut payload = payload_value;

    if T::is_user_signed() {
        let signing_chain = signing_chain
            .ok_or_else(|| "signing_chain must be set for user-signed actions".to_string())?;
        let obj = payload
            .as_object_mut()
            .ok_or_else(|| "action payload must be object".to_string())?;
        obj.insert(
            "signatureChainId".to_string(),
            serde_json::Value::String(format!(
                "0x{:x}",
                signing_chain.signature_chain_id()
            )),
        );
        obj.insert(
            "hyperliquidChain".to_string(),
            serde_json::Value::String(signing_chain.get_hyperliquid_chain()),
        );
        if T::uses_time() && obj.contains_key("nonce") && !obj.contains_key("time") {
            if let Some(nonce_value) = obj.remove("nonce") {
                obj.insert("time".to_string(), nonce_value);
            }
        }
    }

    let mut action_obj = serde_json::Map::new();
    action_obj.insert(
        "type".to_string(),
        serde_json::Value::String(T::action_type().to_string()),
    );

    if T::payload_key() == T::action_type() {
        let payload = payload
            .as_object()
            .cloned()
            .ok_or_else(|| "action payload must be object".to_string())?;
        for (key, value) in payload {
            action_obj.insert(key, value);
        }
    } else {
        action_obj.insert(T::payload_key().to_string(), payload);
    }

    Ok(serde_json::Value::Object(action_obj))
}

struct SigSer<'a>(&'a Signature);

impl<'a> Serialize for SigSer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serialize_sig(self.0, serializer)
    }
}

fn deserialize_action<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Action + DeserializeOwned,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    let obj = value
        .as_object()
        .ok_or_else(|| serde::de::Error::custom("action must be object"))?;

    let action_type = obj
        .get("type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| serde::de::Error::custom("missing action type"))?;

    if action_type != T::action_type() {
        return Err(serde::de::Error::custom(format!(
            "invalid action type: {}",
            action_type
        )));
    }
    let action_payload = if T::payload_key() == T::action_type() {
        let mut payload = obj.clone();
        payload.remove("type");
        if T::is_user_signed()
            && T::uses_time()
            && payload.contains_key("time")
            && !payload.contains_key("nonce")
        {
            if let Some(time_value) = payload.get("time").cloned() {
                payload.insert("nonce".to_string(), time_value);
            }
        }
        serde_json::Value::Object(payload)
    } else {
        let payload = obj
            .get(T::payload_key())
            .ok_or_else(|| serde::de::Error::custom("missing action payload"))?;
        if T::is_user_signed() {
            let mut payload = payload
                .as_object()
                .cloned()
                .ok_or_else(|| serde::de::Error::custom("action payload must be object"))?;
            if T::uses_time() && payload.contains_key("time") && !payload.contains_key("nonce") {
                if let Some(time_value) = payload.get("time").cloned() {
                    payload.insert("nonce".to_string(), time_value);
                }
            }
            serde_json::Value::Object(payload)
        } else {
            payload.clone()
        }
    };

    serde_json::from_value(action_payload).map_err(serde::de::Error::custom)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(bound(deserialize = "T: Action + DeserializeOwned"))]
struct SignedActionHelper<T: Action> {
    #[serde(deserialize_with = "deserialize_action")]
    action: T,
    nonce: u64,
    #[serde(deserialize_with = "deserialize_sig")]
    signature: Signature,
    vault_address: Option<Address>,
    expires_after: Option<u64>,
}

impl<T: Action + Serialize> Serialize for SignedAction<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let action_value = build_action_value(&self.action, self.signing_chain.as_ref())
            .map_err(SerError::custom)?;
        let mut state = serializer.serialize_struct("SignedAction", 5)?;
        state.serialize_field("action", &action_value)?;
        state.serialize_field("nonce", &self.nonce)?;
        state.serialize_field("signature", &SigSer(&self.signature))?;
        if let Some(vault_address) = &self.vault_address {
            state.serialize_field("vaultAddress", vault_address)?;
        }
        if let Some(expires_after) = &self.expires_after {
            state.serialize_field("expiresAfter", expires_after)?;
        }
        state.end()
    }
}

impl<'de, T: Action + DeserializeOwned> Deserialize<'de> for SignedAction<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let helper = SignedActionHelper::deserialize(deserializer)?;
        Ok(SignedAction {
            action: helper.action,
            nonce: helper.nonce,
            signature: helper.signature,
            vault_address: helper.vault_address,
            expires_after: helper.expires_after,
            signing_chain: None,
        })
    }
}

pub fn serialize_sig<S>(sig: &Signature, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut state = serializer.serialize_struct("Signature", 3)?;
    state.serialize_field("r", &format!("0x{:064x}", sig.r()))?;
    state.serialize_field("s", &format!("0x{:064x}", sig.s()))?;
    state.serialize_field("v", &(27 + sig.v() as u64))?;
    state.end()
}

pub fn deserialize_sig<'de, D>(deserializer: D) -> Result<Signature, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    let obj = value
        .as_object()
        .ok_or_else(|| serde::de::Error::custom("signature must be object"))?;

    let r = obj
        .get("r")
        .and_then(|v| v.as_str())
        .ok_or_else(|| serde::de::Error::custom("missing signature.r"))?;
    let s = obj
        .get("s")
        .and_then(|v| v.as_str())
        .ok_or_else(|| serde::de::Error::custom("missing signature.s"))?;

    let v_value = obj
        .get("v")
        .ok_or_else(|| serde::de::Error::custom("missing signature.v"))?;
    let v = if let Some(v) = v_value.as_u64() {
        v
    } else if let Some(v) = v_value.as_str() {
        v.parse::<u64>()
            .map_err(|e| serde::de::Error::custom(e.to_string()))?
    } else {
        return Err(serde::de::Error::custom("invalid signature.v"));
    };

    let r = r.strip_prefix("0x").unwrap_or(r);
    let s = s.strip_prefix("0x").unwrap_or(s);

    let r = U256::from_str_radix(r, 16).map_err(|e| serde::de::Error::custom(e.to_string()))?;
    let s = U256::from_str_radix(s, 16).map_err(|e| serde::de::Error::custom(e.to_string()))?;
    let v = v
        .checked_sub(27)
        .ok_or_else(|| serde::de::Error::custom("invalid v value"))?;

    Ok(Signature::new(r, s, v != 0))
}

pub(crate) fn ser_lowercase<S>(address: &Address, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&address.to_string().to_lowercase())
}

impl<T: Action + DeserializeOwned> SignedAction<T> {
    /// Deserialize from the exchange API format
    pub fn from_json(json: &str) -> Result<Self, Error> {
        serde_json::from_str(json).map_err(|e| Error::JsonParse(e.to_string()))
    }
}

// ============================================================================
// Client (Prepare, Sign, Send)
// ============================================================================

/// Client for preparing, signing, and sending action_v2 requests.
///
/// # Example
/// ```no_run
/// use std::str::FromStr;
///
/// use alloy::primitives::Address;
/// use alloy::signers::local::PrivateKeySigner;
/// use hl_rs::{BaseUrl, exchange::action_v2::{ExchangeActionV2Client, UsdSend}};
/// use rust_decimal_macros::dec;
///
/// # async fn run() -> Result<(), hl_rs::Error> {
/// let client = ExchangeActionV2Client::new(BaseUrl::Testnet);
/// let wallet = PrivateKeySigner::from_str("0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef")?;
///
/// let action = UsdSend::builder()
///             .destination(Address::ZERO)
///             .amount(dec!(1.0))
///             .build()?;
///
/// let _response = client.execute_action(action, &wallet).await?;
/// # Ok(())
/// # }
/// ```
///
/// # Example (perpDeploy setSubDeployers)
/// ```no_run
/// use std::str::FromStr;
///
/// use alloy::primitives::Address;
/// use alloy::signers::local::PrivateKeySigner;
/// use hl_rs::exchange::action_v2::{ExchangeActionV2Client, PerpDeployAction, SetSubDeployers, SubDeployer, Variant};
/// use hl_rs::BaseUrl;
///
/// # async fn run() -> Result<(), hl_rs::Error> {
/// let wallet = PrivateKeySigner::from_str("0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef")?;
/// let client = ExchangeActionV2Client::new(BaseUrl::Testnet).with_signer(wallet);
///
/// let sub_deployer = SubDeployer::enable(Address::ZERO, Variant::SetOracle);
/// let action = PerpDeployAction::set_sub_deployers("km", vec![sub_deployer]);
/// let _response = client.execute_action(action).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct ExchangeActionV2Client {
    base_url: BaseUrl,
    http_client: HttpClient,
    vault_address: Option<Address>,
    expires_after: Option<u64>,
    signer_private_key: Option<PrivateKeySigner>,
}

impl ExchangeActionV2Client {
    pub fn new(base_url: BaseUrl) -> Self {
        let http_client = HttpClient {
            client: Client::default(),
            base_url: base_url.get_url(),
        };

        Self {
            base_url,
            http_client,
            vault_address: None,
            expires_after: None,
            signer_private_key: None,
        }
    }

    pub fn with_vault_address(mut self, vault_address: Address) -> Self {
        self.vault_address = Some(vault_address);
        self
    }

    pub fn with_expires_after(mut self, expires_after: u64) -> Self {
        self.expires_after = Some(expires_after);
        self
    }
    pub fn with_signer(mut self, signer_private_key: PrivateKeySigner) -> Self {
        self.signer_private_key = Some(signer_private_key);
        self
    }

    pub fn prepare_action<A: Action>(&self, action: A) -> Result<PreparedAction<A>, Error> {
        prepare_action(
            action,
            self.base_url.get_signing_chain(),
            self.vault_address,
            self.expires_after,
        )
    }

    pub fn sign_action<A: Action>(
        &self,
        action: A,
        wallet: &PrivateKeySigner,
    ) -> Result<SignedAction<A>, Error> {
        self.prepare_action(action)?.sign(wallet)
    }

    pub async fn send_signed_action<A: Action + Serialize>(
        &self,
        signed_action: SignedAction<A>,
    ) -> Result<crate::exchange::responses::ExchangeResponse, Error> {
        let output = self.http_client.post("/exchange", signed_action).await?;
        let raw: crate::exchange::responses::ExchangeResponseStatusRaw =
            serde_json::from_str(&output).map_err(|e| Error::JsonParse(e.to_string()))?;
        raw.into_result()
    }

    pub async fn send_action<A: Action + Serialize>(
        &self,
        action: A,
    ) -> Result<crate::exchange::responses::ExchangeResponse, Error> {
        let prepared = self.prepare_action(action)?;
        let signed = prepared.sign(self.signer_private_key.as_ref().unwrap())?;
        let ser_signed = serde_json::to_string_pretty(&signed).unwrap();
        self.send_signed_action(signed).await
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Get current timestamp in milliseconds
pub fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

/// Prepare an action for signing
pub fn prepare_action<A: Action>(
    action: A,
    signing_chain: &SigningChain,
    vault_address: Option<Address>,
    expires_after: Option<u64>,
) -> Result<PreparedAction<A>, Error> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use super::perp_deploy_v2::SetOpenInterestCaps;
    use rust_decimal_macros::dec;
    use serde_json::json;

    #[test]
    fn test_enable_big_blocks_serialization() {
        let action = ToggleBigBlocks::enable();
        let signing_chain = SigningChain::Mainnet;

        let prepared = prepare_action(action, &signing_chain, None, None).unwrap();

        // Create a dummy signature for testing
        let sig = Signature::new(U256::from(1), U256::from(2), false);
        let signed = prepared.with_signature(sig);

        let json = serde_json::to_string_pretty(&signed).unwrap();
        println!("{}", json);

        // Verify structure
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.get("action").is_some());
        assert!(parsed.get("nonce").is_some());
        assert!(parsed.get("signature").is_some());

        let action_obj = parsed.get("action").unwrap();
        assert_eq!(action_obj.get("type").unwrap(), "evmUserModify");
        assert!(action_obj.get("evmUserModify").is_some());
    }

    #[test]
    fn test_usd_send_serialization() {
        let action = UsdSend::new(Address::ZERO, dec!(100.0));
        let signing_chain = SigningChain::Testnet;

        let prepared = prepare_action(action, &signing_chain, None, None).unwrap();

        let sig = Signature::new(U256::from(1), U256::from(2), false);
        let signed = prepared.with_signature(sig);

        let json = serde_json::to_string_pretty(&signed).unwrap();
        println!("{}", json);

        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let action_obj = parsed.get("action").unwrap();
        assert_eq!(action_obj.get("type").unwrap(), "usdSend");
        assert_eq!(action_obj.get("hyperliquidChain").unwrap(), "Testnet");
        assert_eq!(action_obj.get("signatureChainId").unwrap(), "0x66eee");
        assert!(action_obj.get("destination").is_some());
        assert!(action_obj.get("amount").is_some());
        assert!(action_obj.get("time").is_some());
    }

    #[test]
    fn test_usd_send_signing_hardcoded() {
        let wallet: PrivateKeySigner =
            "e908f86dbb4d55ac876378565aafeabc187f6690f046459397b17d9b9a19688e"
                .parse()
                .unwrap();
        let destination =
            "0x0d1d9635d0640821d15e323ac8adadfa9c111414".parse().unwrap();
        let action = UsdSend {
            destination,
            amount: dec!(1.0),
            nonce: Some(1700000000000),
        };
        let signing_chain = SigningChain::Testnet;
        let meta = SigningMeta {
            nonce: 0,
            vault_address: None,
            expires_after: None,
            signing_chain: &signing_chain,
        };

        let signing_hash = action.signing_hash(&meta).unwrap();
        let signature = wallet.sign_hash_sync(&signing_hash).unwrap();
        let actual_sig = signature.to_string();

        let expected_sig = "0xd4ad4602ed7f14be3960934f4e2ed205f0f24539fc467a606999719a0f13c384521699107322321581048d35fed5d3272eb913377a42940299922c9a854fad281b";
        assert_eq!(actual_sig, expected_sig);
        assert_eq!(
            format!("{:#x}", signature.r()).to_lowercase(),
            "0xd4ad4602ed7f14be3960934f4e2ed205f0f24539fc467a606999719a0f13c384"
        );
        assert_eq!(
            format!("{:#x}", signature.s()).to_lowercase(),
            "0x521699107322321581048d35fed5d3272eb913377a42940299922c9a854fad28"
        );
        let v = if signature.v() { 28 } else { 27 };
        assert_eq!(v, 27);
    }

    #[test]
    fn test_perp_deploy_v2_set_open_interest_caps_serialization_matches_docs() {
        let action = SetOpenInterestCaps {
            caps: vec![("BTC".to_string(), 1_000_000), ("ETH".to_string(), 500_000)],
            nonce: None,
        };
        let signing_chain = SigningChain::Testnet;
        let prepared = prepare_action(action, &signing_chain, None, None).unwrap();
        let sig = Signature::new(U256::from(3), U256::from(4), true);
        let signed = prepared.with_signature(sig);

        let parsed = serde_json::to_value(&signed).unwrap();
        let action_obj = parsed.get("action").unwrap();
        assert_eq!(
            action_obj,
            &json!({
                "type": "perpDeploy",
                "setOpenInterestCaps": [
                    ["BTC", 1_000_000],
                    ["ETH", 500_000]
                ]
            })
        );
    }

    #[test]
    fn test_extract_action_kind_preserves_values() {
        // Test UsdSend
        let dest = Address::repeat_byte(0x42);
        let amount = dec!(123.456);
        let nonce = 9876543210u64;
        let usd_send = UsdSend {
            destination: dest,
            amount,
            nonce: Some(nonce),
        };

        let signing_chain = SigningChain::Testnet;
        let prepared = prepare_action(usd_send, &signing_chain, None, None).unwrap();
        let sig = Signature::new(U256::from(1), U256::from(2), false);
        let signed = prepared.with_signature(sig);

        // Extract ActionKind and verify values are preserved
        let kind = signed.extract_action_kind();
        match kind {
            ActionKind::UsdSend(extracted) => {
                assert_eq!(extracted.destination, dest);
                assert_eq!(extracted.amount, amount);
                assert_eq!(extracted.nonce, Some(nonce));
            }
            _ => panic!("Expected ActionKind::UsdSend"),
        }

        // Test EnableBigBlocks
        let enable_blocks = ToggleBigBlocks::enable();
        let prepared = prepare_action(enable_blocks, &signing_chain, None, None).unwrap();
        let sig = Signature::new(U256::from(3), U256::from(4), true);
        let signed = prepared.with_signature(sig);

        let kind = signed.extract_action_kind();
        match kind {
            ActionKind::ToggleBigBlocks(extracted) => {
                assert!(extracted.using_big_blocks);
            }
            _ => panic!("Expected ActionKind::EnableBigBlocks"),
        }

        // Also test disable variant
        let disable_blocks = ToggleBigBlocks::disable();
        let prepared = prepare_action(disable_blocks, &signing_chain, None, None).unwrap();
        let sig = Signature::new(U256::from(5), U256::from(6), false);
        let signed = prepared.with_signature(sig);

        let kind = signed.extract_action_kind();
        match kind {
            ActionKind::ToggleBigBlocks(extracted) => {
                assert!(!extracted.using_big_blocks);
            }
            _ => panic!("Expected ActionKind::EnableBigBlocks"),
        }
    }

    #[test]
    fn test_signed_action_serialization_roundtrip() {
        // Test UsdSend roundtrip
        let dest = Address::repeat_byte(0xAB);
        let amount = dec!(999.123);
        let nonce = 1234567890u64;
        let usd_send = UsdSend {
            destination: dest,
            amount,
            nonce: Some(nonce),
        };

        let signing_chain = SigningChain::Testnet;
        let prepared = prepare_action(usd_send, &signing_chain, None, None).unwrap();
        let nonce = prepared.nonce;

        let r = U256::from_str_radix("abcd1234", 16).unwrap();
        let s = U256::from_str_radix("5678ef90", 16).unwrap();
        let sig = Signature::new(r, s, true);
        let signed = prepared.with_signature(sig);

        // Serialize to JSON
        let json = serde_json::to_string(&signed).unwrap();
        println!("Serialized UsdSend: {}", json);

        // Deserialize back
        let deserialized: SignedAction<UsdSend> = SignedAction::from_json(&json).unwrap();

        // Verify all fields match
        assert_eq!(deserialized.action.destination, dest);
        assert_eq!(deserialized.action.amount, amount);
        assert_eq!(deserialized.action.nonce, Some(nonce));
        assert_eq!(deserialized.nonce, nonce);
        assert_eq!(deserialized.signature.r(), signed.signature.r());
        assert_eq!(deserialized.signature.s(), signed.signature.s());
        assert_eq!(deserialized.signature.v(), signed.signature.v());
        assert_eq!(deserialized.vault_address, None);
        assert_eq!(deserialized.expires_after, None);

        // Test EnableBigBlocks roundtrip
        let enable_blocks = ToggleBigBlocks::enable();
        let prepared = prepare_action(enable_blocks, &signing_chain, None, None).unwrap();
        let nonce = prepared.nonce;

        let sig = Signature::new(U256::from(111), U256::from(222), false);
        let signed = prepared.with_signature(sig);

        let json = serde_json::to_string(&signed).unwrap();
        println!("Serialized EnableBigBlocks: {}", json);

        let deserialized: SignedAction<ToggleBigBlocks> = SignedAction::from_json(&json).unwrap();

        assert!(deserialized.action.using_big_blocks);
        assert_eq!(deserialized.nonce, nonce);
        assert_eq!(deserialized.signature.r(), signed.signature.r());
        assert_eq!(deserialized.signature.s(), signed.signature.s());

        // Test with optional fields (vault_address)
        let vault = Address::repeat_byte(0x99);
        let usd_send = UsdSend {
            destination: dest,
            amount: dec!(50.0),
            nonce: Some(111222333),
        };
        let prepared = prepare_action(usd_send, &signing_chain, Some(vault), None).unwrap();
        let sig = Signature::new(U256::from(333), U256::from(444), true);
        let signed = prepared.with_signature(sig);

        let json = serde_json::to_string(&signed).unwrap();
        println!("Serialized with vault: {}", json);

        let deserialized: SignedAction<UsdSend> = SignedAction::from_json(&json).unwrap();
        assert_eq!(deserialized.vault_address, Some(vault));
    }

    //#[test]
    //fn test_perp_deploy_register_asset_serialization_matches_docs() {
    //    let action = PerpDeployAction::new(RegisterAsset {
    //        max_gas: None,
    //        asset_request: RegisterAssetRequest {
    //            coin: "BTC".to_string(),
    //            sz_decimals: 8,
    //            oracle_px: "100".to_string(),
    //            margin_table_id: 1,
    //            only_isolated: true,
    //        },
    //        dex: "HL".to_string(),
    //        schema: Some(PerpDexSchemaInput {
    //            full_name: "Hyperliquid".to_string(),
    //            collateral_token: 1,
    //            oracle_updater: Some("0x0000000000000000000000000000000000000000".to_string()),
    //        }),
    //    });

    //    let signing_chain = SigningChain::Testnet;
    //    let prepared = prepare_action(action, &signing_chain, None, None).unwrap();
    //    let sig = Signature::new(U256::from(1), U256::from(2), false);
    //    let signed = prepared.with_signature(sig);

    //    let parsed = serde_json::to_value(&signed).unwrap();
    //    let action_obj = parsed.get("action").unwrap();
    //    assert_eq!(
    //        action_obj,
    //        &json!({
    //            "type": "perpDeploy",
    //            "registerAsset": {
    //                "maxGas": null,
    //                "assetRequest": {
    //                    "coin": "BTC",
    //                    "szDecimals": 8,
    //                    "oraclePx": "100",
    //                    "marginTableId": 1,
    //                    "onlyIsolated": true
    //                },
    //                "dex": "HL",
    //                "schema": {
    //                    "fullName": "Hyperliquid",
    //                    "collateralToken": 1,
    //                    "oracleUpdater": "0x0000000000000000000000000000000000000000"
    //                }
    //            }
    //        })
    //    );
    //}

    //#[test]
    //fn test_perp_deploy_set_open_interest_caps_serialization_matches_docs() {
    //    let action = PerpDeployAction::new(SetOpenInterestCaps {
    //        caps: vec![("BTC".to_string(), 1_000_000), ("ETH".to_string(), 500_000)],
    //    });

    //    let signing_chain = SigningChain::Testnet;
    //    let prepared = prepare_action(action, &signing_chain, None, None).unwrap();
    //    let sig = Signature::new(U256::from(3), U256::from(4), true);
    //    let signed = prepared.with_signature(sig);

    //    let parsed = serde_json::to_value(&signed).unwrap();
    //    let action_obj = parsed.get("action").unwrap();
    //    assert_eq!(
    //        action_obj,
    //        &json!({
    //            "type": "perpDeploy",
    //            "setOpenInterestCaps": [
    //                ["BTC", 1_000_000],
    //                ["ETH", 500_000]
    //            ]
    //        })
    //    );
    //}

    //#[test]
    //fn test_perp_deploy_set_sub_deployers_serialization_matches_docs() {
    //    let action = PerpDeployAction::set_sub_deployers(
    //        "HL",
    //        vec![SubDeployer {
    //            variant: Variant::SetOracle,
    //            user: Address::ZERO,
    //            allowed: true,
    //        }],
    //    );

    //    let signing_chain = SigningChain::Testnet;
    //    let prepared = prepare_action(action, &signing_chain, None, None).unwrap();
    //    let sig = Signature::new(U256::from(5), U256::from(6), false);
    //    let signed = prepared.with_signature(sig);

    //    let parsed = serde_json::to_value(&signed).unwrap();
    //    let action_obj = parsed.get("action").unwrap();
    //    assert_eq!(
    //        action_obj,
    //        &json!({
    //            "type": "perpDeploy",
    //            "setSubDeployers": {
    //                "dex": "HL",
    //                "subDeployers": [
    //                    { "variant": "setOracle", "user": "0x0000000000000000000000000000000000000000", "allowed": true }
    //                ]
    //            }
    //        })
    //    );
    //}

    #[test]
    fn test_multisig_signing_hash_consistent_l1() {
        let signing_chain = SigningChain::Testnet;
        let meta = SigningMeta {
            nonce: 123456,
            vault_address: None,
            expires_after: None,
            signing_chain: &signing_chain,
        };

        let action = ToggleBigBlocks::enable();
        let payload_multi_sig_user = Address::repeat_byte(0x11);
        let outer_signer = Address::repeat_byte(0x22);

        let hash1 = action
            .multisig_signing_hash(&meta, payload_multi_sig_user, outer_signer)
            .unwrap();
        let hash2 = action
            .multisig_signing_hash(&meta, payload_multi_sig_user, outer_signer)
            .unwrap();

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_multisig_signing_hash_consistent_user_signed() {
        let signing_chain = SigningChain::Testnet;
        let meta = SigningMeta {
            nonce: 424242,
            vault_address: None,
            expires_after: None,
            signing_chain: &signing_chain,
        };

        let action = UsdSend {
            destination: Address::repeat_byte(0x33),
            amount: dec!(10.5),
            nonce: Some(424242),
        };
        let payload_multi_sig_user = Address::repeat_byte(0x44);
        let outer_signer = Address::repeat_byte(0x55);

        let hash1 = action
            .multisig_signing_hash(&meta, payload_multi_sig_user, outer_signer)
            .unwrap();
        let hash2 = action
            .multisig_signing_hash(&meta, payload_multi_sig_user, outer_signer)
            .unwrap();

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_dec_ser() {
        assert_eq!("10000.5".to_string(), dec!(10_000.5).to_string());
    }
}
