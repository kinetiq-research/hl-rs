// ============================================================================
// Client (Prepare, Sign, Send)
// ============================================================================

use alloy::{primitives::Address, signers::local::PrivateKeySigner};
use reqwest::Client;
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    actions::{Action, SignedAction},
    clients::exchange::responses::{ExchangeResponse, ExchangeResponseStatusRaw},
    http::HttpClient,
    BaseUrl, Error, PreparedAction,
};

/// Client for preparing, signing, and sending exchange actions.
///
/// # Example
/// ```no_run
/// use std::str::FromStr;
///
/// use alloy::primitives::Address;
/// use alloy::signers::local::PrivateKeySigner;
/// use hl_rs::{BaseUrl, ExchangeClientV2, UsdSend};
/// use rust_decimal_macros::dec;
///
/// # async fn run() -> Result<(), hl_rs::Error> {
/// let wallet =
///     PrivateKeySigner::from_str("0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef")
///         .expect("valid private key hex");
/// let client = ExchangeClientV2::new(BaseUrl::Testnet).with_signer(wallet);
///
/// let action = UsdSend::new(Address::ZERO, dec!(1.0));
///
/// let _response = client.send_action(action).await?;
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
/// use hl_rs::{BaseUrl, ExchangeClientV2, SetSubDeployers, SubDeployer, SubDeployerVariant};
///
/// # async fn run() -> Result<(), hl_rs::Error> {
/// let wallet =
///     PrivateKeySigner::from_str("0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef")
///         .expect("valid private key hex");
/// let client = ExchangeClientV2::new(BaseUrl::Testnet).with_signer(wallet);
///
/// let sub_deployer = SubDeployer::enable(Address::ZERO, SubDeployerVariant::SetOracle);
/// let action = SetSubDeployers::new("km", vec![sub_deployer]);
/// let _response = client.send_action(action).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct ExchangeClient {
    base_url: BaseUrl,
    http_client: HttpClient,
    vault_address: Option<Address>,
    expires_after: Option<u64>,
    signer_private_key: Option<PrivateKeySigner>,
    nonce_counter: Arc<AtomicU64>,
}

impl ExchangeClient {
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
            nonce_counter: Arc::new(AtomicU64::new(Self::current_timestamp_ms())),
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
        PreparedAction::new(
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
        self.prepare_action(self.ensure_action_nonce(action))?.sign(wallet)
    }

    pub async fn send_signed_action<A: Action + Serialize>(
        &self,
        signed_action: SignedAction<A>,
    ) -> Result<ExchangeResponse, Error> {
        let output = self.http_client.post("/exchange", signed_action).await?;
        let raw: ExchangeResponseStatusRaw =
            serde_json::from_str(&output).map_err(|e| Error::JsonParse(e.to_string()))?;

        raw.into_result()
    }

    pub async fn send_action<A: Action + Serialize>(
        &self,
        action: A,
    ) -> Result<ExchangeResponse, Error> {
        let prepared = self.prepare_action(self.ensure_action_nonce(action))?;
        let signer = self.signer_private_key.as_ref().ok_or(Error::SignerNotSet)?;
        let signed = prepared.sign(signer)?;
        self.send_signed_action(signed).await
    }

    /// Ensure the action carries a nonce before signing. If the caller already
    /// set one, respect it; otherwise assign a strictly-monotonic nonce so
    /// rapid-fire actions (e.g. UpdateLeverage then an order) never collide.
    fn ensure_action_nonce<A: Action>(&self, action: A) -> A {
        if action.nonce().is_some() {
            action
        } else {
            action.with_nonce(self.next_nonce())
        }
    }

    /// Monotonic nonce: current ms, bumped by 1 if we'd otherwise repeat or go
    /// backwards within the same millisecond. Lock-free via a CAS loop.
    fn next_nonce(&self) -> u64 {
        let now = Self::current_timestamp_ms();
        loop {
            let last = self.nonce_counter.load(Ordering::Relaxed);
            let next = if now > last { now } else { last + 1 };
            if self
                .nonce_counter
                .compare_exchange(last, next, Ordering::SeqCst, Ordering::Relaxed)
                .is_ok()
            {
                return next;
            }
        }
    }

    fn current_timestamp_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
}
