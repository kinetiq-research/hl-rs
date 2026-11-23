//! Full `/exchange` bodies, signature included, against hyperliquid-python-sdk.
//!
//! `tests/vectors/signed_body.json` (written by `tests/vectors/gen_signed_body.py`) holds
//! `cancel` bodies the SDK signed with the test key 0x1111...11, picked so that `r` in one
//! case and `s` in the other has a leading zero nibble. The SDK writes both unpadded; a padded
//! rendering would change the body. Each case is signed here with the same key through the
//! public API, and the serialised `SignedL1Action` must equal the SDK body. The SDK always
//! sends `vaultAddress` and `expiresAfter`, as `null` when unset; hl-rs leaves them out, so
//! those `null` keys are dropped before comparing.

use alloy::signers::local::PrivateKeySigner;
use hl_rs::{L1ActionKind, SignedActionKind, SigningChain};
use serde_json::{json, Value};

const VECTORS: &str = include_str!("vectors/signed_body.json");
const KEY: &str = "0x1111111111111111111111111111111111111111111111111111111111111111";

fn without_nulls(body: &Value) -> Value {
    let mut out = body.as_object().unwrap().clone();
    out.retain(|_, v| !v.is_null());
    Value::Object(out)
}

#[test]
fn signed_bodies_match_the_sdk() {
    let vectors: Value = serde_json::from_str(VECTORS).unwrap();
    let wallet: PrivateKeySigner = KEY.parse().unwrap();
    assert_eq!(
        format!("{:?}", wallet.address()).to_lowercase(),
        vectors["signer"].as_str().unwrap().to_lowercase()
    );
    let cases = vectors["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 2);
    for case in cases {
        let name = case["name"].as_str().unwrap();
        assert_eq!(case["chain"], "mainnet", "{name}");
        let sdk = &case["body"];
        let nonce = sdk["nonce"].as_u64().unwrap();
        let envelope = json!({
            "action": sdk["action"],
            "nonce": nonce,
            "signature": {"r": "0x1", "s": "0x2", "v": 27},
        });
        let parsed = SignedActionKind::from_json(&envelope.to_string()).unwrap();
        let kind = L1ActionKind::try_from(parsed.action)
            .unwrap_or_else(|other| panic!("{name}: not an L1 action: {other:?}"))
            .with_nonce(nonce);
        let signed = kind
            .prepare(&SigningChain::Mainnet, None, None)
            .unwrap()
            .sign(&wallet)
            .unwrap();
        let ours = serde_json::to_value(&signed).unwrap();
        assert_eq!(ours, without_nulls(sdk), "{name}");
    }
}

#[test]
fn the_vectors_exercise_short_r_and_short_s() {
    let vectors: Value = serde_json::from_str(VECTORS).unwrap();
    let short = |case: &Value, field: &str| {
        case["body"]["signature"][field].as_str().unwrap().len() < 2 + 64
    };
    let cases = vectors["cases"].as_array().unwrap();
    assert!(cases.iter().any(|c| short(c, "r")));
    assert!(cases.iter().any(|c| short(c, "s")));
}
