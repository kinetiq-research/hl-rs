//! `userOutcome` bodies against the dicts HyperCore received.
//!
//! `tests/vectors/user_outcome.json` holds each body as sent to HyperCore testnet on 2026-11-23
//! (two accepted from an approved agent, two whose signer HyperCore recovered) and two documented
//! shapes, with the msgpack CPython packs and the SDK's `Agent` digests on both chains.
//! `tests/vectors/gen_user_outcome.py` writes the file.
//!
//! Each case goes through the public API in both directions: the dict parses through
//! `SignedActionKind::from_json` into the right `L1ActionKind` variant and hashes to the SDK digest,
//! its `/exchange` body is the dict, and the same action built by the constructors hashes the same.

use alloy::hex;
use hl_rs::{
    ActionKind, L1ActionKind, MergeOutcome, MergeQuestion, NegateOutcome, SignedActionKind,
    SigningChain, SplitOutcome,
};
use rust_decimal::Decimal;
use serde_json::{json, Value};

const VECTORS: &str = include_str!("vectors/user_outcome.json");

fn cases() -> Vec<Value> {
    let file: Value = serde_json::from_str(VECTORS).unwrap();
    file["cases"].as_array().unwrap().clone()
}

fn parsed(case: &Value) -> L1ActionKind {
    let envelope = json!({
        "action": case["action"],
        "nonce": case["nonce"],
        "signature": {"r": "0x1", "s": "0x2", "v": 27},
    });
    let signed = SignedActionKind::from_json(&envelope.to_string()).unwrap();
    L1ActionKind::try_from(signed.action).expect("an L1 action")
}

fn digest(kind: &L1ActionKind, chain: &SigningChain) -> String {
    format!("{:#x}", kind.signing_hash(chain, None, None).unwrap())
}

fn nonce(case: &Value) -> u64 {
    case["nonce"].as_u64().unwrap()
}

#[test]
fn every_case_hashes_to_the_sdk_digest_and_serialises_as_sent() {
    let cases = cases();
    assert_eq!(cases.len(), 6);
    for case in &cases {
        let name = case["name"].as_str().unwrap();
        let kind = parsed(case).with_nonce(nonce(case));
        assert_eq!(kind.action_type(), "userOutcome", "{name}");
        assert_eq!(
            digest(&kind, &SigningChain::Mainnet),
            case["digest_mainnet"].as_str().unwrap(),
            "{name} mainnet"
        );
        assert_eq!(
            digest(&kind, &SigningChain::Testnet),
            case["digest_testnet"].as_str().unwrap(),
            "{name} testnet"
        );
        let signed = kind
            .prepare(&SigningChain::Testnet, None, None)
            .unwrap()
            .with_signature(alloy_signer::Signature::new(
                alloy::primitives::U256::from(1),
                alloy::primitives::U256::from(2),
                false,
            ));
        let body: Value = serde_json::to_value(&signed).unwrap();
        assert_eq!(body["action"], case["action"], "{name} body");
        // Key order is part of the hash: compare the serialised text, not the parsed values.
        assert_eq!(
            serde_json::to_string(&body["action"]).unwrap(),
            serde_json::to_string(&case["action"]).unwrap(),
            "{name} key order"
        );
    }
}

/// The constructors build the measured bodies byte for byte.
#[test]
fn the_constructors_build_the_measured_bodies() {
    let cases = cases();
    let built: [L1ActionKind; 6] = [
        SplitOutcome::new(11074, Decimal::new(10, 1)).into(),
        MergeOutcome::all(11074).into(),
        MergeQuestion::all(1283).into(),
        NegateOutcome::new(1283, 17209, Decimal::new(10, 1)).into(),
        MergeOutcome::new(20115, Decimal::new(5, 1)).into(),
        MergeQuestion::new(820, Decimal::new(1230, 1)).into(),
    ];
    for (case, kind) in cases.iter().zip(built) {
        let name = case["name"].as_str().unwrap();
        let kind = kind.with_nonce(nonce(case));
        assert_eq!(
            digest(&kind, &SigningChain::Testnet),
            case["digest_testnet"].as_str().unwrap(),
            "{name}"
        );
        assert_eq!(
            digest(&kind, &SigningChain::Mainnet),
            case["digest_mainnet"].as_str().unwrap(),
            "{name}"
        );
    }
}

/// Each payload key reads back as its own variant, and a body carrying two keys is refused
/// rather than read as either.
#[test]
fn the_payload_key_picks_the_variant() {
    let cases = cases();
    let kinds: Vec<&str> = cases
        .iter()
        .map(|c| match parsed(c) {
            L1ActionKind::SplitOutcome(_) => "split",
            L1ActionKind::MergeOutcome(_) => "merge",
            L1ActionKind::MergeQuestion(_) => "mergeQuestion",
            L1ActionKind::NegateOutcome(_) => "negate",
            other => panic!("parsed as {other:?}"),
        })
        .collect();
    assert_eq!(
        kinds,
        [
            "split",
            "merge",
            "mergeQuestion",
            "negate",
            "merge",
            "mergeQuestion"
        ]
    );

    let both = json!({
        "action": {"type": "userOutcome",
                   "splitOutcome": {"outcome": 1, "amount": "1.0"},
                   "mergeOutcome": {"outcome": 1, "amount": null}},
        "nonce": 1,
        "signature": {"r": "0x1", "s": "0x2", "v": 27},
    });
    assert!(SignedActionKind::from_json(&both.to_string()).is_err());
}

/// The msgpack each body is hashed from, against CPython `msgpack.packb` of the dict.
#[test]
fn the_msgpack_is_the_python_packing() {
    for case in cases() {
        let name = case["name"].as_str().unwrap();
        let action = SignedActionKind::from_json(
            &json!({"action": case["action"], "nonce": 1,
                    "signature": {"r": "0x1", "s": "0x2", "v": 27}})
            .to_string(),
        )
        .unwrap()
        .action;
        let packed = match &action {
            ActionKind::SplitOutcome(a) => packed(a),
            ActionKind::MergeOutcome(a) => packed(a),
            ActionKind::MergeQuestion(a) => packed(a),
            ActionKind::NegateOutcome(a) => packed(a),
            other => panic!("{name} parsed as {other:?}"),
        };
        assert_eq!(packed, case["msgpack"].as_str().unwrap(), "{name}");
    }
}

/// `{"type":"userOutcome", <payloadKey>: body}` packed as Python packs the dict.
fn packed<T: serde::Serialize + hl_rs::Action>(body: &T) -> String {
    struct Wrapped<'a, T>(&'a T);
    impl<T: serde::Serialize + hl_rs::Action> serde::Serialize for Wrapped<'_, T> {
        fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            use serde::ser::SerializeMap;
            let mut map = s.serialize_map(Some(2))?;
            map.serialize_entry("type", T::ACTION_TYPE)?;
            map.serialize_entry(T::PAYLOAD_KEY, self.0)?;
            map.end()
        }
    }
    format!(
        "0x{}",
        hex::encode(rmp_serde::to_vec_named(&Wrapped(body)).unwrap())
    )
}
