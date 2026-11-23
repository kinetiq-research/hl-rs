//! L1 action signing hashes against hyperliquid-python-sdk.
//!
//! `tests/vectors/l1_wire.json` holds 68 L1 actions, each with the action
//! dict, the nonce, the msgpack the SDK hashes, and the EIP-712 `Agent`
//! digest an agent key signs on mainnet and on testnet. Cases marked
//! `sdk-method` come from the SDK's own `Exchange` methods (0.21.0,
//! `be7523d`); cases marked `hand-built` have no SDK method and carry the
//! body shape HyperCore recovered the signer of on testnet.
//!
//! 24 of them are the order family, written by
//! `tests/vectors/gen_order_family.py`: `order` (each tif, with and without
//! a cloid, prices 0.1 and 3000, size 1.5) and two-to-three-order batches,
//! `cancel` and `cancelByCloid` (single and batch, with and without the fast
//! flag), `batchModify` by cloid and by oid, and `borrowLend` supply and
//! withdraw with an amount and with `null`.
//!
//! Each case goes through the public API only: the dict is read with
//! `SignedActionKind::from_json` and narrowed to an `L1ActionKind` with the
//! case nonce, and `L1ActionKind::signing_hash` must equal the SDK digest on
//! both chains. The `/exchange` body of the `SignedL1Action` must equal the
//! SDK dict, and its msgpack must equal the SDK bytes. `tests/l1_action_kind.rs`
//! checks that `L1ActionKind` hashes and serialises as `PreparedAction` does
//! for every concrete type.

use alloy::hex;
use hl_rs::{L1ActionKind, SignedActionKind, SigningChain};
use serde_json::{json, Value};

const VECTORS: &str = include_str!("vectors/l1_wire.json");

/// A placeholder signature: the body under test is the action, not the signature.
const SIGNATURE: &str = r#"{"r":"0x1","s":"0x2","v":27}"#;

struct Hashed {
    mainnet: String,
    testnet: String,
    action: Value,
}

/// Reads the dict as an `L1ActionKind` with the case nonce; a user-signed or unknown
/// action is an error.
fn l1_kind(action: &Value, nonce: u64) -> Result<(L1ActionKind, SignedActionKind), String> {
    let envelope = json!({
        "action": action,
        "nonce": nonce,
        "signature": serde_json::from_str::<Value>(SIGNATURE).unwrap(),
    });
    let signed = SignedActionKind::from_json(&envelope.to_string()).map_err(|e| e.to_string())?;
    let kind = L1ActionKind::try_from(signed.action.clone())
        .map_err(|other| format!("not an L1 action: {other:?}"))?;
    Ok((kind.with_nonce(nonce), signed))
}

fn hash_case(action: &Value, nonce: u64) -> Result<Hashed, String> {
    let (kind, signed) = l1_kind(action, nonce)?;
    let digest = |chain: &SigningChain| {
        kind.signing_hash(chain, None, None)
            .map(|hash| format!("{hash:#x}"))
            .map_err(|e| e.to_string())
    };
    let prepared = kind
        .clone()
        .prepare(&SigningChain::Mainnet, None, None)
        .map_err(|e| e.to_string())?;
    let body = serde_json::to_value(prepared.with_signature(signed.signature)).unwrap();
    Ok(Hashed {
        mainnet: digest(&SigningChain::Mainnet)?,
        testnet: digest(&SigningChain::Testnet)?,
        action: body["action"].clone(),
    })
}

fn check(case: &Value) -> Result<(), String> {
    let nonce = case["nonce"].as_u64().ok_or("nonce")?;
    let got = hash_case(&case["action"], nonce)?;
    let mut errors = Vec::new();
    if got.mainnet != case["digest_mainnet"].as_str().unwrap() {
        errors.push(format!("mainnet digest {}", got.mainnet));
    }
    if got.testnet != case["digest_testnet"].as_str().unwrap() {
        errors.push(format!("testnet digest {}", got.testnet));
    }
    if got.action != case["action"] {
        errors.push(format!("body action {}", got.action));
    }
    let msgpack = format!(
        "0x{}",
        hex::encode(rmp_serde::to_vec_named(&got.action).unwrap())
    );
    if msgpack != case["msgpack"].as_str().unwrap() {
        errors.push(format!("body msgpack {msgpack}"));
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

fn cases() -> Vec<Value> {
    let file: Value = serde_json::from_str(VECTORS).unwrap();
    file["cases"].as_array().unwrap().clone()
}

#[test]
fn l1_wire_vectors_match_the_sdk_on_both_chains() {
    let cases = cases();
    assert_eq!(cases.len(), 68, "vector file changed size");
    let failures: Vec<String> = cases
        .iter()
        .filter_map(|case| {
            check(case).err().map(|e| {
                format!(
                    "{} / {}: {e}",
                    case["kind"].as_str().unwrap(),
                    case["name"].as_str().unwrap()
                )
            })
        })
        .collect();
    assert!(
        failures.is_empty(),
        "{} of {} cases differ:\n{}",
        failures.len(),
        cases.len(),
        failures.join("\n")
    );
}
