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
use derive_builder::Builder;
use hl_rs_derive::{L1Action, UserSignedAction};
use reqwest::Client;
use rust_decimal::Decimal;
use serde::{
    de::DeserializeOwned,
    ser::{SerializeMap, SerializeStruct},
    Deserialize, Deserializer, Serialize, Serializer,
};

use crate::{http::HttpClient, BaseUrl, Error, SigningChain};

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

// Use to convert serialized SignedAction<T> to a concrete type.
impl_action_kind!(UsdSend, EnableBigBlocks);

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

    /// Whether to exclude vault_address from the hash (default: false)
    /// Override to `true` for actions like PerpDeploy
    const EXCLUDE_VAULT_FROM_HASH: bool = false;
}

/// Compute the L1 action hash (MessagePack + metadata)
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
    /// Action type name for API serialization
    fn action_type(&self) -> &'static str;

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
        }
    }
}

/// Fully signed action ready to submit
///
/// Serializes to the exchange API format:
/// ```json
/// {
///   "action": { "type": "actionType", "actionType": { ...payload... } },
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
    pub vault_address: Option<Address>,
    pub expires_after: Option<u64>,
    pub signature: Signature,
}

impl<T: Action> SignedAction<T> {
    /// Extract the action kind (clones the inner action into an ActionKind variant)
    pub fn extract_action_kind(&self) -> ActionKind {
        self.action.extract_action_kind()
    }
}

impl<T: Action> Serialize for SignedAction<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Count fields: action, nonce, signature, plus optionals
        let mut field_count = 3;
        if self.vault_address.is_some() {
            field_count += 1;
        }
        if self.expires_after.is_some() {
            field_count += 1;
        }

        let mut map = serializer.serialize_map(Some(field_count))?;

        // Serialize action with type tag
        map.serialize_entry("action", &ActionWrapper::from(&self.action))?;

        // Serialize nonce
        map.serialize_entry("nonce", &self.nonce)?;

        // Serialize signature
        let sig_value = serialize_sig_value(&self.signature)
            .map_err(|e| serde::ser::Error::custom(e.to_string()))?;
        map.serialize_entry("signature", &sig_value)?;

        // Serialize optional fields
        if let Some(vault) = &self.vault_address {
            map.serialize_entry("vaultAddress", &vault.to_string())?;
        }
        if let Some(expires) = &self.expires_after {
            map.serialize_entry("expiresAfter", expires)?;
        }

        map.end()
    }
}

/// Helper for serializing action with type tag
struct ActionWrapper<'a, T: Serialize> {
    action_type: &'a str,
    action: &'a T,
}

impl<'a, T: Action + Serialize> From<&'a T> for ActionWrapper<'a, T> {
    fn from(action: &'a T) -> Self {
        Self {
            action_type: action.action_type(),
            action: &action,
        }
    }
}

impl<'a, T: Serialize> Serialize for ActionWrapper<'a, T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("type", self.action_type)?;
        map.serialize_entry(self.action_type, self.action)?;
        map.end()
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

fn serialize_sig_value(sig: &Signature) -> Result<serde_json::Value, serde_json::Error> {
    serialize_sig(sig, serde_json::value::Serializer)
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

impl<T: Action + DeserializeOwned> SignedAction<T> {
    /// Deserialize from the exchange API format
    pub fn from_json(json: &str) -> Result<Self, Error> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct RawSignedAction {
            action: serde_json::Value,
            nonce: u64,
            #[serde(deserialize_with = "deserialize_sig")]
            signature: Signature,
            vault_address: Option<String>,
            expires_after: Option<u64>,
        }

        let raw: RawSignedAction =
            serde_json::from_str(json).map_err(|e| Error::JsonParse(e.to_string()))?;

        // Extract action type and payload
        let action_obj = raw
            .action
            .as_object()
            .ok_or_else(|| Error::JsonParse("action must be object".to_string()))?;

        let action_type = action_obj
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::JsonParse("missing action type".to_string()))?;

        let action_payload = action_obj
            .get(action_type)
            .ok_or_else(|| Error::JsonParse("missing action payload".to_string()))?;

        let action: T = serde_json::from_value(action_payload.clone())
            .map_err(|e| Error::JsonParse(e.to_string()))?;

        // Signature already deserialized via helper
        let signature = raw.signature;

        // Parse vault address
        let vault_address = raw
            .vault_address
            .map(|s| s.parse::<Address>())
            .transpose()
            .map_err(|e| Error::GenericParse(e.to_string()))?;

        Ok(SignedAction {
            action,
            nonce: raw.nonce,
            vault_address,
            expires_after: raw.expires_after,
            signature,
        })
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
#[derive(Debug, Clone)]
pub struct ExchangeActionV2Client {
    base_url: BaseUrl,
    http_client: HttpClient,
    vault_address: Option<Address>,
    expires_after: Option<u64>,
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

    pub async fn send_action<A: Action + Serialize>(
        &self,
        signed_action: SignedAction<A>,
    ) -> Result<crate::exchange::responses::ExchangeResponse, Error> {
        let output = self.http_client.post("/exchange", signed_action).await?;
        let raw: crate::exchange::responses::ExchangeResponseStatusRaw =
            serde_json::from_str(&output).map_err(|e| Error::JsonParse(e.to_string()))?;
        raw.into_result()
    }

    pub async fn execute_action<A: Action + Serialize>(
        &self,
        action: A,
        wallet: &PrivateKeySigner,
    ) -> Result<crate::exchange::responses::ExchangeResponse, Error> {
        let prepared = self.prepare_action(action)?;
        let signed = prepared.sign(wallet)?;
        self.send_action(signed).await
    }
}

// ============================================================================
// Example Action Implementations
// ============================================================================

/// Enable big blocks mode for EVM user (L1 Action example)
#[derive(Debug, Clone, Serialize, Deserialize, Builder, L1Action)]
#[action(action_type = "evmUserModify")]
#[serde(rename_all = "camelCase")]
#[builder(setter(into))]
pub struct EnableBigBlocks {
    pub using_big_blocks: bool,
    #[builder(default)]
    pub nonce: Option<u64>,
}

impl EnableBigBlocks {
    pub fn builder() -> EnableBigBlocksBuilder {
        EnableBigBlocksBuilder::default()
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
#[action(types = "UsdSend(string hyperliquidChain,string destination,string amount,uint64 nonce)")]
#[serde(rename_all = "camelCase")]
#[builder(setter(into))]
pub struct UsdSend {
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

// UserSignedAction + Action impls derived via macro.

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
        signing_hash,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_enable_big_blocks_serialization() {
        let action = EnableBigBlocks::enable();
        let signing_chain = SigningChain::Testnet;

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
        assert!(action_obj.get("usdSend").is_some());
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
        let enable_blocks = EnableBigBlocks::enable();
        let prepared = prepare_action(enable_blocks, &signing_chain, None, None).unwrap();
        let sig = Signature::new(U256::from(3), U256::from(4), true);
        let signed = prepared.with_signature(sig);

        let kind = signed.extract_action_kind();
        match kind {
            ActionKind::EnableBigBlocks(extracted) => {
                assert!(extracted.using_big_blocks);
            }
            _ => panic!("Expected ActionKind::EnableBigBlocks"),
        }

        // Also test disable variant
        let disable_blocks = EnableBigBlocks::disable();
        let prepared = prepare_action(disable_blocks, &signing_chain, None, None).unwrap();
        let sig = Signature::new(U256::from(5), U256::from(6), false);
        let signed = prepared.with_signature(sig);

        let kind = signed.extract_action_kind();
        match kind {
            ActionKind::EnableBigBlocks(extracted) => {
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
        let enable_blocks = EnableBigBlocks::enable();
        let prepared = prepare_action(enable_blocks, &signing_chain, None, None).unwrap();
        let nonce = prepared.nonce;

        let sig = Signature::new(U256::from(111), U256::from(222), false);
        let signed = prepared.with_signature(sig);

        let json = serde_json::to_string(&signed).unwrap();
        println!("Serialized EnableBigBlocks: {}", json);

        let deserialized: SignedAction<EnableBigBlocks> = SignedAction::from_json(&json).unwrap();

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

    #[test]
    fn test_multisig_signing_hash_consistent_l1() {
        let signing_chain = SigningChain::Testnet;
        let meta = SigningMeta {
            nonce: 123456,
            vault_address: None,
            expires_after: None,
            signing_chain: &signing_chain,
        };

        let action = EnableBigBlocks::enable();
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
}
