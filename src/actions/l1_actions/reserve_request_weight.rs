use alloy::primitives::Address;
use hl_rs_derive::L1Action;
use serde::{Deserialize, Serialize, Serializer};

/// Reserve additional request weight (rate-limit capacity).
///
/// Instead of trading to raise the address-based rate limit, this action reserves additional
/// actions directly. It is paid from the signer's **perp balance** at `0.0005 USDC` per unit of
/// `weight` (so `weight = floor(usdc / 0.0005)`).
///
/// See [Reserve additional actions](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/exchange-endpoint#reserve-additional-actions).
///
/// # Example
/// ```no_run
/// use std::str::FromStr;
///
/// use alloy::signers::local::PrivateKeySigner;
/// use hl_rs::{BaseUrl, ExchangeClient, ReserveRequestWeight};
///
/// # async fn run() -> Result<(), hl_rs::Error> {
/// let wallet = PrivateKeySigner::from_str(
///     "0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
/// )
/// .expect("valid private key hex");
/// let client = ExchangeClient::new(BaseUrl::Testnet).with_signer(wallet);
///
/// let action = ReserveRequestWeight::new(1_000);
/// let _response = client.send_action(action).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, L1Action)]
#[action(action_type = "reserveRequestWeight")]
#[serde(rename_all = "camelCase")]
pub struct ReserveRequestWeight {
    /// Number of additional actions to reserve.
    pub weight: u64,
    /// The L1 user whose weight is reserved, when paying for someone other than the signer. The
    /// docs show the field; most callers leave it `None`.
    ///
    /// **`None` is the absent key, never `null`.** Measured on testnet 2026-11-20: the body
    /// `{"type":"reserveRequestWeight","weight":1}` verified to the master and was charged, while
    /// the same body with `"destination":null` mis-recovered to a random address, for an agent's
    /// signature and the master's alike. HyperCore does not hash a null `destination` as sent.
    ///
    /// `Some` is sent as the lowercase address, after `weight`. Measured on testnet 2026-11-20 with
    /// a key approved nowhere: `{"type":"reserveRequestWeight","weight":1,"destination":"0xaaf0…"}`
    /// answered `Must deposit … User: <that key>`, so the signature recovered and HyperCore has the
    /// field. What a reservation for another user does was not measured.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "ser_lowercase_opt"
    )]
    pub destination: Option<Address>,
    #[serde(skip_serializing)]
    pub nonce: Option<u64>,
}

fn ser_lowercase_opt<S: Serializer>(a: &Option<Address>, s: S) -> Result<S::Ok, S::Error> {
    match a {
        Some(a) => s.serialize_str(&format!("{a:#x}")),
        None => s.serialize_none(),
    }
}

impl ReserveRequestWeight {
    pub fn new(weight: u64) -> Self {
        Self {
            weight,
            destination: None,
            nonce: None,
        }
    }

    /// Reserve the weight for `destination` rather than the signer.
    ///
    /// The wire form is measured to recover the signer (see [`Self::destination`]); the effect,
    /// whose balance pays and whose limit rises, is not.
    pub fn for_destination(mut self, destination: Address) -> Self {
        self.destination = Some(destination);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::{PreparedAction, SignedActionKind};
    use crate::SigningChain;
    use alloy::primitives::U256;
    use alloy_signer::Signature;
    use serde_json::json;

    #[test]
    fn action_shape_matches_docs() {
        let action = ReserveRequestWeight::new(1234);
        let prepared = PreparedAction::new(action, &SigningChain::Testnet, None, None).unwrap();
        let signed = prepared.with_signature(Signature::new(U256::from(1), U256::from(2), false));

        let v = serde_json::to_value(&signed).unwrap();
        assert_eq!(
            v.get("action").unwrap(),
            &json!({ "type": "reserveRequestWeight", "weight": 1234 })
        );
    }

    fn msgpack_hex(a: &ReserveRequestWeight) -> String {
        alloy::hex::encode(
            rmp_serde::to_vec_named(&crate::actions::L1ActionWrapper { action: a }).unwrap(),
        )
    }

    /// The body HyperCore verified and charged on testnet (2026-11-20), against CPython
    /// `msgpack.packb`: no `destination` key, and in particular no `null`.
    #[test]
    fn no_destination_is_the_absent_key() {
        let a = ReserveRequestWeight::new(1);
        assert_eq!(serde_json::to_string(&a).unwrap(), r#"{"weight":1}"#);
        assert_eq!(
            msgpack_hex(&a),
            "82a474797065b47265736572766552657175657374576569676874a677656967687401"
        );
    }

    /// With a destination: after `weight`, as a lowercase hex string.
    #[test]
    fn a_destination_is_a_lowercase_address_after_the_weight() {
        let a = ReserveRequestWeight::new(1).for_destination(Address::repeat_byte(0x22));
        assert_eq!(
            serde_json::to_string(&a).unwrap(),
            r#"{"weight":1,"destination":"0x2222222222222222222222222222222222222222"}"#
        );
        assert_eq!(
            msgpack_hex(&a),
            "83a474797065b47265736572766552657175657374576569676874a677656967687401ab64657374696e6174696f6ed92a307832323232323232323232323232323232323232323232323232323232323232323232323232323232"
        );
    }

    #[test]
    fn round_trips_through_signed_action_kind() {
        let action = ReserveRequestWeight::new(42);
        let prepared = PreparedAction::new(action, &SigningChain::Testnet, None, None).unwrap();
        let signed = prepared.with_signature(Signature::new(U256::from(3), U256::from(4), true));
        let json = serde_json::to_string(&signed).unwrap();

        match SignedActionKind::from_json(&json).unwrap().action {
            crate::actions::ActionKind::ReserveRequestWeight(a) => assert_eq!(a.weight, 42),
            other => panic!("expected ReserveRequestWeight, got {other:?}"),
        }
    }
}
