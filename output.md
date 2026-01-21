# Proposed Action Type System

## Overview

This design uses Rust's trait system to create a clean separation between:
- **L1 actions** - Signed via connection_id hash (MessagePack serialization)
- **User-signed actions** - Signed via EIP-712 typed data

The `Action` trait is implemented via blanket impls for both, ensuring compile-time correctness.

---

## Core Types

```rust
use alloy::{
    dyn_abi::Eip712Domain,
    primitives::{keccak256, Address, B256, Signature},
    signers::{local::PrivateKeySigner, SignerSync},
    sol,
    sol_types::{eip712_domain, SolStruct, SolValue},
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::Error;

/// Network-specific signing parameters
#[derive(Debug, Clone)]
pub enum SigningChain {
    Mainnet,
    Testnet,
}

impl SigningChain {
    /// L1 action source identifier
    pub fn source(&self) -> &'static str {
        match self {
            SigningChain::Mainnet => "a",
            SigningChain::Testnet => "b",
        }
    }

    /// Chain name for user-signed actions
    pub fn hyperliquid_chain(&self) -> &'static str {
        match self {
            SigningChain::Mainnet => "Mainnet",
            SigningChain::Testnet => "Testnet",
        }
    }

    /// EIP-712 signature chain ID (Arbitrum)
    pub fn signature_chain_id(&self) -> u64 {
        match self {
            SigningChain::Mainnet => 42161,
            SigningChain::Testnet => 421614,
        }
    }
}

/// Metadata passed during action signing
#[derive(Debug, Clone)]
pub struct SigningMeta {
    pub nonce: u64,
    pub vault_address: Option<Address>,
    pub expires_after: Option<u64>,
    pub signing_chain: SigningChain,
}

/// EIP-712 Agent struct for L1 action signing
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
```

---

## L1Action Trait

```rust
/// Trait for actions signed via L1 connection_id mechanism.
///
/// L1 actions are hashed using MessagePack serialization combined with
/// timestamp, vault address, and expiration metadata.
///
/// # Examples
/// - Orders, Cancels, Leverage updates
/// - Vault transfers, Referrer settings
/// - PerpDeploy operations
pub trait L1Action: Serialize + Send + Sync + 'static {
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
    let mut bytes = rmp_serde::to_vec_named(action)
        .map_err(|e| Error::Serialization(e.to_string()))?;

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
```

---

## UserSignedAction Trait

```rust
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

    /// Get the embedded timestamp/nonce from this action
    fn timestamp(&self) -> u64;

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
```

---

## Action Trait (Unified Interface)

```rust
/// Unified trait for all exchange actions.
///
/// This trait is automatically implemented for types that implement
/// either `L1Action` or `UserSignedAction` via blanket implementations.
/// Client code uses this trait for a uniform API.
pub trait Action: Send + Sync {
    /// Action type name for API serialization
    fn action_type(&self) -> &'static str;

    /// Serialize the action payload to JSON
    fn to_payload(&self) -> Result<serde_json::Value, Error>;

    /// Build the signing hash from action and metadata
    fn signing_hash(&self, meta: &SigningMeta) -> Result<B256, Error>;

    /// Get embedded timestamp if this is a user-signed action
    fn embedded_timestamp(&self) -> Option<u64> {
        None
    }

    /// Whether this is an L1 action (vs user-signed)
    fn is_l1_action(&self) -> bool;
}

// Blanket implementation: L1Action -> Action
impl<T: L1Action> Action for T {
    fn action_type(&self) -> &'static str {
        T::ACTION_TYPE
    }

    fn to_payload(&self) -> Result<serde_json::Value, Error> {
        serde_json::to_value(self)
            .map_err(|e| Error::Serialization(e.to_string()))
    }

    fn signing_hash(&self, meta: &SigningMeta) -> Result<B256, Error> {
        let vault_for_hash = if T::EXCLUDE_VAULT_FROM_HASH {
            None
        } else {
            meta.vault_address
        };

        let connection_id = compute_l1_hash(
            self,
            meta.nonce,
            vault_for_hash,
            meta.expires_after,
        )?;

        Ok(agent_signing_hash(connection_id, meta.signing_chain.source()))
    }

    fn is_l1_action(&self) -> bool {
        true
    }
}

// Blanket implementation: UserSignedAction -> Action
impl<T: UserSignedAction> Action for T {
    fn action_type(&self) -> &'static str {
        T::ACTION_TYPE
    }

    fn to_payload(&self) -> Result<serde_json::Value, Error> {
        serde_json::to_value(self)
            .map_err(|e| Error::Serialization(e.to_string()))
    }

    fn signing_hash(&self, meta: &SigningMeta) -> Result<B256, Error> {
        Ok(self.eip712_signing_hash(&meta.signing_chain))
    }

    fn embedded_timestamp(&self) -> Option<u64> {
        Some(self.timestamp())
    }

    fn is_l1_action(&self) -> bool {
        false
    }
}
```

---

## Example: UsdSend (UserSignedAction)

```rust
use rust_decimal::Decimal;

/// USDC transfer between Hyperliquid accounts
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsdSend {
    pub destination: Address,
    #[serde(with = "decimal_str")]
    pub amount: Decimal,
    pub time: u64,
}

impl UsdSend {
    pub fn new(destination: Address, amount: Decimal, time: u64) -> Self {
        Self { destination, amount, time }
    }
}

impl UserSignedAction for UsdSend {
    const ACTION_TYPE: &'static str = "usdSend";

    fn timestamp(&self) -> u64 {
        self.time
    }

    fn struct_hash(&self, chain: &SigningChain) -> B256 {
        let type_hash = keccak256(
            "HyperliquidTransaction:UsdSend(string hyperliquidChain,string destination,string amount,uint64 time)"
        );

        let items = (
            type_hash,
            keccak256(chain.hyperliquid_chain()),
            keccak256(self.destination.to_lowercase().to_string()),
            keccak256(self.amount.to_string()),
            self.time,
        );

        keccak256(items.abi_encode())
    }
}
```

---

## Example: EnableBigBlocks (L1Action)

```rust
/// Enable big blocks mode for EVM user
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnableBigBlocks {
    pub using_big_blocks: bool,
}

impl EnableBigBlocks {
    pub fn enable() -> Self {
        Self { using_big_blocks: true }
    }

    pub fn disable() -> Self {
        Self { using_big_blocks: false }
    }
}

impl L1Action for EnableBigBlocks {
    const ACTION_TYPE: &'static str = "evmUserModify";
}
```

---

## Prepared and Signed Action Types

```rust
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
    pub fn sign(self, wallet: &PrivateKeySigner) -> Result<SignedAction, Error> {
        let signature = wallet
            .sign_hash_sync(&self.signing_hash)
            .map_err(|e| Error::SignatureFailure(e.to_string()))?;

        Ok(SignedAction {
            action_type: self.action.action_type().to_string(),
            payload: self.action.to_payload()?,
            nonce: self.nonce,
            vault_address: self.vault_address,
            expires_after: self.expires_after,
            signature,
        })
    }

    /// Attach an externally-provided signature
    pub fn with_signature(self, signature: Signature) -> Result<SignedAction, Error> {
        Ok(SignedAction {
            action_type: self.action.action_type().to_string(),
            payload: self.action.to_payload()?,
            nonce: self.nonce,
            vault_address: self.vault_address,
            expires_after: self.expires_after,
            signature,
        })
    }
}

/// Fully signed action ready to submit
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignedAction<T: Action> {
    #[serde(rename = "type")]
    pub action_type: String,
    pub action: T,
    pub nonce: u64,
    pub vault_address: Option<Address>,
    pub expires_after: Option<u64>,
    pub signature: Signature,
}
```

---

## Exchange Client

```rust
use reqwest::Client;

/// HTTP client for Hyperliquid Exchange API
pub struct ExchangeClient {
    http: Client,
    base_url: String,
    signing_chain: SigningChain,
    vault_address: Option<Address>,
    expires_after: Option<u64>,
}

impl ExchangeClient {
    pub fn new(base_url: impl Into<String>, signing_chain: SigningChain) -> Self {
        Self {
            http: Client::new(),
            base_url: base_url.into(),
            signing_chain,
            vault_address: None,
            expires_after: None,
        }
    }

    /// Set vault address for all subsequent actions
    pub fn with_vault(mut self, vault: Address) -> Self {
        self.vault_address = Some(vault);
        self
    }

    /// Set expiration for all subsequent actions
    pub fn with_expires_after(mut self, expires: u64) -> Self {
        self.expires_after = Some(expires);
        self
    }

    /// Prepare an action for signing
    pub fn prepare<A: Action>(&self, action: A) -> Result<PreparedAction<A>, Error> {
        // For user-signed actions, use embedded timestamp
        // For L1 actions, generate fresh nonce
        let nonce = action
            .embedded_timestamp()
            .unwrap_or_else(current_timestamp_ms);

        let meta = SigningMeta {
            nonce,
            vault_address: self.vault_address,
            expires_after: self.expires_after,
            signing_chain: self.signing_chain.clone(),
        };

        let signing_hash = action.signing_hash(&meta)?;

        Ok(PreparedAction {
            action,
            nonce,
            vault_address: self.vault_address,
            expires_after: self.expires_after,
            signing_hash,
        })
    }

    /// Submit a signed action to the exchange
    pub async fn send(&self, signed: SignedAction) -> Result<ExchangeResponse, Error> {
        let response = self
            .http
            .post(format!("{}/exchange", self.base_url))
            .json(&signed)
            .send()
            .await
            .map_err(|e| Error::Http(e.to_string()))?;

        let status = response.status();
        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| Error::Http(e.to_string()))?;

        if status.is_success() {
            Ok(ExchangeResponse::Success(body))
        } else {
            Ok(ExchangeResponse::Error {
                code: status.as_u16(),
                message: body.to_string(),
            })
        }
    }

    /// Convenience: prepare, sign, and send in one call
    pub async fn execute<A: Action>(
        &self,
        action: A,
        wallet: &PrivateKeySigner,
    ) -> Result<ExchangeResponse, Error> {
        let prepared = self.prepare(action)?;
        let signed = prepared.sign(wallet)?;
        self.send(signed).await
    }

}

fn format_signature(sig: &Signature) -> serde_json::Value {
    serde_json::json!({
        "r": format!("0x{:064x}", sig.r()),
        "s": format!("0x{:064x}", sig.s()),
        "v": 27 + sig.v() as u64
    })
}

fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

#[derive(Debug)]
pub enum ExchangeResponse {
    Success(serde_json::Value),
    Error { code: u16, message: String },
}
```

---

## Usage Examples

### Send USDC (User-Signed Action)

```rust
use rust_decimal_macros::dec;

let client = ExchangeClient::new("https://api.hyperliquid.xyz", SigningChain::Mainnet);

// Create the action
let transfer = UsdSend::new(
    "0x1234...".parse().unwrap(),
    dec!(100.50),
    current_timestamp_ms(),
);

// Option 1: Execute in one call
let response = client.execute(transfer, &wallet).await?;

// Option 2: Prepare, inspect, then sign
let prepared = client.prepare(transfer)?;
println!("Signing hash: {:?}", prepared.signing_hash());
let signed = prepared.sign(&wallet)?;
let response = client.send(signed).await?;
```

### Enable Big Blocks (L1 Action)

```rust
let client = ExchangeClient::new("https://api.hyperliquid.xyz", SigningChain::Mainnet);

let action = EnableBigBlocks::enable();

// Same API - the trait system handles signing differences
let response = client.execute(action, &wallet).await?;
```

### External Signing Workflow

```rust
let prepared = client.prepare(action)?;

// Get hash for hardware wallet / MPC / external signer
let hash = prepared.signing_hash();

// ... obtain signature externally ...
let signature: Signature = external_sign(hash).await?;

// Attach signature and send
let signed = prepared.with_signature(signature)?;
let response = client.send(signed).await?;
```

---

## Adding New Actions

### New L1 Action

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateLeverage {
    pub asset: u32,
    pub is_cross: bool,
    pub leverage: u32,
}

// Just one impl block!
impl L1Action for UpdateLeverage {
    const ACTION_TYPE: &'static str = "updateLeverage";
}
```

### New User-Signed Action

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Withdraw {
    pub destination: Address,
    #[serde(with = "decimal_str")]
    pub amount: Decimal,
    pub time: u64,
}

impl UserSignedAction for Withdraw {
    const ACTION_TYPE: &'static str = "withdraw3";

    fn timestamp(&self) -> u64 {
        self.time
    }

    fn struct_hash(&self, chain: &SigningChain) -> B256 {
        let type_hash = keccak256(
            "HyperliquidTransaction:Withdraw(string hyperliquidChain,string destination,string amount,uint64 time)"
        );

        let items = (
            type_hash,
            keccak256(chain.hyperliquid_chain()),
            keccak256(self.destination.to_lowercase().to_string()),
            keccak256(self.amount.to_string()),
            self.time,
        );

        keccak256(items.abi_encode())
    }
}
```

---

## Serde Helpers

```rust
/// Serialize/deserialize Decimal as string
pub mod decimal_str {
    use rust_decimal::Decimal;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(val: &Decimal, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&val.to_string())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Decimal, D::Error> {
        let s = String::deserialize(d)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}
```

---

## Design Summary

| Component | Purpose |
|-----------|---------|
| `L1Action` | Marker trait + `const ACTION_TYPE` for L1 actions |
| `UserSignedAction` | EIP-712 signing with `struct_hash()` + `timestamp()` |
| `Action` | Unified interface via blanket impls, provides `signing_hash()` |
| `SigningMeta` | Carries nonce, vault, expires_after, chain |
| `PreparedAction<A>` | Generic wrapper with precomputed signing hash |
| `SignedAction` | Type-erased, ready to send |
| `ExchangeClient` | Stateful client with `prepare()`, `send()`, `execute()` |

**Key benefits:**
- Compile-time correctness: blanket impls ensure proper signing
- Minimal boilerplate: new L1 action = struct + 1 const
- Uniform client API: same `execute()` call for all actions
- Flexible signing: supports local wallets and external signers
- Type-safe decimals: `rust_decimal::Decimal` throughout
- Typed addresses: `alloy::primitives::Address`