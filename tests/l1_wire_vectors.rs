//! L1 action signing hashes against hyperliquid-python-sdk.
//!
//! `tests/vectors/l1_wire.json` holds 44 L1 actions, each with the action
//! dict, the nonce, the msgpack the SDK hashes, and the EIP-712 `Agent`
//! digest an agent key signs on mainnet and on testnet. Cases marked
//! `sdk-method` come from the SDK's own `Exchange` methods (0.21.0,
//! `be7523d`); cases marked `hand-built` have no SDK method and carry the
//! body shape HyperCore recovered the signer of on testnet.
//!
//! Each case goes through the public API only: the dict is read with
//! `SignedActionKind::from_json`, the concrete action gets the case nonce,
//! and `PreparedAction::signing_hash` must equal the SDK digest on both
//! chains. The `/exchange` body built from the prepared action must equal
//! the SDK dict, and its msgpack must equal the SDK bytes.

use alloy::hex;
use hl_rs::{Action, ActionKind, PreparedAction, SignedActionKind, SigningChain};
use serde_json::{json, Value};

const VECTORS: &str = include_str!("vectors/l1_wire.json");

/// A placeholder signature: the body under test is the action, not the signature.
const SIGNATURE: &str = r#"{"r":"0x1","s":"0x2","v":27}"#;

struct Hashed {
    mainnet: String,
    testnet: String,
    action: Value,
}

fn signed_kind(action: &Value, nonce: u64) -> Result<SignedActionKind, String> {
    let envelope = json!({
        "action": action,
        "nonce": nonce,
        "signature": serde_json::from_str::<Value>(SIGNATURE).unwrap(),
    });
    SignedActionKind::from_json(&envelope.to_string()).map_err(|e| e.to_string())
}

fn hash<A: Action + Clone>(action: A, nonce: u64, signed: SignedActionKind) -> Hashed {
    let digest = |chain: &SigningChain| {
        let prepared =
            PreparedAction::new(action.clone().with_nonce(nonce), chain, None, None).unwrap();
        format!("{:#x}", prepared.signing_hash())
    };
    let prepared = PreparedAction::new(
        action.clone().with_nonce(nonce),
        &SigningChain::Mainnet,
        None,
        None,
    )
    .unwrap();
    let body = serde_json::to_value(prepared.with_signature(signed.signature)).unwrap();
    Hashed {
        mainnet: digest(&SigningChain::Mainnet),
        testnet: digest(&SigningChain::Testnet),
        action: body["action"].clone(),
    }
}

/// Hashes every L1 `ActionKind` variant; a user-signed or unknown action is an error.
macro_rules! hash_l1 {
    ($signed:expr, $nonce:expr; $($variant:ident),* $(,)?) => {{
        let signed: SignedActionKind = $signed;
        match signed.action.clone() {
            $(ActionKind::$variant(a) => Ok(hash(a, $nonce, signed)),)*
            other => Err(format!("not an L1 action: {other:?}")),
        }
    }};
}

fn hash_case(action: &Value, nonce: u64) -> Result<Hashed, String> {
    let signed = signed_kind(action, nonce)?;
    hash_l1!(signed, nonce;
        ToggleBigBlocks, ReserveRequestWeight, NoOp, ClaimRewards, SetReferrer,
        CreateSubAccount, SubAccountTransfer, SubAccountSpotTransfer, VaultTransfer,
        BorrowLend, AgentSendAsset, AgentEnableDexAbstraction, FinalizeEvmContract,
        UpdateLeverage, UpdateIsolatedMargin, ScheduleCancel,
        BatchOrder, BatchCancel, CancelByCloid, BatchModify,
        RegisterAsset, SetOpenInterestCaps, ToggleTrading, SetOracle, SetFeeRecipient,
        SetFeeScale, SetFundingMultipliers, SetFundingInterestRates, SetFundingClamps,
        SetMarginTableIds, InsertMarginTable, MigrateDexQuoteToken, DisableDex,
        SetSubDeployers, SetGrowthModes, SetMarginModes, SetPerpAnnotation,
        PerpSetDeployerFees,
        RegisterToken, Genesis, UserGenesis, RegisterSpot, RegisterHyperliquidity,
        SetDeployerFees, RequestEvmContract, EnableFreezePrivilege, FreezeUser,
        RevokeFreezePrivilege, EnableQuoteToken, DisableQuoteToken, SetDeployerLabel,
        SetTokenAnnotation,
    )
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
    assert_eq!(cases.len(), 44, "vector file changed size");
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
