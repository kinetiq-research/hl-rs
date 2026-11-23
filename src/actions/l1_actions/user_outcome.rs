//! `userOutcome`: split, merge and negate the share tokens of an outcome market.
//!
//! One action type with four payloads, discriminated by which key the body carries:
//!
//! ```json
//! {"type":"userOutcome","splitOutcome":{"outcome":11074,"amount":"1.0"}}
//! {"type":"userOutcome","mergeOutcome":{"outcome":11074,"amount":null}}
//! {"type":"userOutcome","mergeQuestion":{"question":1283,"amount":null}}
//! {"type":"userOutcome","negateOutcome":{"question":1283,"outcome":17209,"amount":"1.0"}}
//! ```
//!
//! Each converts value inside the signing account: quote tokens into an outcome's Yes and No
//! shares, the shares back into quote tokens, or one outcome's No shares into Yes shares of the
//! question's other outcomes. None of them names a second account.
//!
//! These are **L1 actions**, signed by an agent (API wallet) for its master. That was measured on
//! testnet on 2026-11-23: an agent's `splitOutcome` of `"1.0"` on outcome 11074 credited the
//! master's spot balance with both side tokens (`+110740`, `+110741`), its `mergeOutcome` with a
//! `null` amount returned the USDC to the master, and the agent's own balances did not change.
//!
//! # Field order is the wire format
//!
//! The body is msgpacked before it is hashed, so the declaration order of each struct's fields is
//! part of the signature. The orders below are the documented ones and the ones HyperCore received
//! in the measurements; `tests/vectors/user_outcome.json` pins each body's msgpack and digests.
//!
//! # Amounts
//!
//! An amount is sent as a string at the scale the [`Decimal`] was built with: `Decimal::new(10, 1)`
//! goes out as `"1.0"`, the form HyperCore accepted and the form its documentation writes
//! (`"123.0"`). Nothing here rescales it, so a caller that wants that form builds it.
//!
//! A `None` amount on the two merges is `null` on the wire, which HyperCore reads as **the maximal
//! amount**, not as nothing. [`MergeOutcome::all`] and [`MergeQuestion::all`] name that case at the
//! call site. A split and a negate have no `null` form.
//!
//! Outcome and question ids are `u32`, the width CoreWriter's `outcomeOperation` (action 17) gives
//! them, and they come from `/info` `outcomeMeta`.

use hl_rs_derive::L1Action;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Split `amount` quote tokens into `amount` Yes and `amount` No shares of `outcome`.
///
/// CoreWriter's `outcomeOperation` with `encodedOperation` 0.
#[derive(Serialize, Deserialize, Debug, Clone, L1Action)]
#[action(action_type = "userOutcome", payload_key = "splitOutcome")]
pub struct SplitOutcome {
    /// Outcome id, from `outcomeMeta`.
    pub outcome: u32,
    /// Quote tokens to split, sent at the `Decimal`'s own scale (see the module docs).
    pub amount: Decimal,
    #[serde(skip_serializing)]
    pub nonce: Option<u64>,
}

/// Merge `amount` Yes and `amount` No shares of `outcome` back into `amount` quote tokens.
///
/// CoreWriter's `outcomeOperation` with `encodedOperation` 1.
#[derive(Serialize, Deserialize, Debug, Clone, L1Action)]
#[action(action_type = "userOutcome", payload_key = "mergeOutcome")]
pub struct MergeOutcome {
    /// Outcome id, from `outcomeMeta`.
    pub outcome: u32,
    /// Share pairs to merge. **`None` is `null` on the wire, which merges the maximal amount**;
    /// use [`MergeOutcome::all`] for it. The key is always sent.
    pub amount: Option<Decimal>,
    #[serde(skip_serializing)]
    pub nonce: Option<u64>,
}

/// Merge `amount` Yes shares of every outcome of `question` into `amount` quote tokens.
///
/// CoreWriter's `outcomeOperation` with `encodedOperation` 2.
#[derive(Serialize, Deserialize, Debug, Clone, L1Action)]
#[action(action_type = "userOutcome", payload_key = "mergeQuestion")]
pub struct MergeQuestion {
    /// Question id, from `outcomeMeta`.
    pub question: u32,
    /// Yes shares per outcome to merge. **`None` is `null` on the wire, which merges the maximal
    /// amount**; use [`MergeQuestion::all`] for it. The key is always sent.
    pub amount: Option<Decimal>,
    #[serde(skip_serializing)]
    pub nonce: Option<u64>,
}

/// Convert `amount` No shares of `outcome` into `amount` Yes shares of every other outcome of
/// `question`.
///
/// CoreWriter's `outcomeOperation` with `encodedOperation` 3.
#[derive(Serialize, Deserialize, Debug, Clone, L1Action)]
#[action(action_type = "userOutcome", payload_key = "negateOutcome")]
pub struct NegateOutcome {
    /// Question id, from `outcomeMeta`.
    pub question: u32,
    /// The outcome whose No shares are converted; one of `question`'s outcomes.
    pub outcome: u32,
    /// No shares to convert, sent at the `Decimal`'s own scale.
    pub amount: Decimal,
    #[serde(skip_serializing)]
    pub nonce: Option<u64>,
}

impl SplitOutcome {
    pub fn new(outcome: u32, amount: Decimal) -> Self {
        Self {
            outcome,
            amount,
            nonce: None,
        }
    }
}

impl MergeOutcome {
    /// Merge `amount` share pairs.
    pub fn new(outcome: u32, amount: Decimal) -> Self {
        Self {
            outcome,
            amount: Some(amount),
            nonce: None,
        }
    }

    /// Merge **every** share pair the account holds of `outcome`: the `null` amount.
    pub fn all(outcome: u32) -> Self {
        Self {
            outcome,
            amount: None,
            nonce: None,
        }
    }
}

impl MergeQuestion {
    /// Merge `amount` Yes shares of each of `question`'s outcomes.
    pub fn new(question: u32, amount: Decimal) -> Self {
        Self {
            question,
            amount: Some(amount),
            nonce: None,
        }
    }

    /// Merge the maximal amount: the `null` amount.
    pub fn all(question: u32) -> Self {
        Self {
            question,
            amount: None,
            nonce: None,
        }
    }
}

impl NegateOutcome {
    pub fn new(question: u32, outcome: u32, amount: Decimal) -> Self {
        Self {
            question,
            outcome,
            amount,
            nonce: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::{Action, L1ActionWrapper};

    fn msgpack<T: Action + Serialize>(action: &T) -> String {
        alloy::hex::encode(rmp_serde::to_vec_named(&L1ActionWrapper { action }).unwrap())
    }

    #[test]
    fn all_four_share_the_action_type() {
        assert_eq!(<SplitOutcome as Action>::ACTION_TYPE, "userOutcome");
        assert_eq!(<MergeOutcome as Action>::ACTION_TYPE, "userOutcome");
        assert_eq!(<MergeQuestion as Action>::ACTION_TYPE, "userOutcome");
        assert_eq!(<NegateOutcome as Action>::ACTION_TYPE, "userOutcome");
        assert_eq!(<SplitOutcome as Action>::PAYLOAD_KEY, "splitOutcome");
        assert_eq!(<MergeOutcome as Action>::PAYLOAD_KEY, "mergeOutcome");
        assert_eq!(<MergeQuestion as Action>::PAYLOAD_KEY, "mergeQuestion");
        assert_eq!(<NegateOutcome as Action>::PAYLOAD_KEY, "negateOutcome");
    }

    /// The accepted split's body, against CPython `msgpack.packb` of the dict HyperCore received
    /// (`tests/vectors/user_outcome.json`, case "split, accepted").
    #[test]
    fn the_accepted_split_packs_as_sent() {
        let a = SplitOutcome::new(11074, Decimal::new(10, 1));
        assert_eq!(
            serde_json::to_string(&a).unwrap(),
            r#"{"outcome":11074,"amount":"1.0"}"#
        );
        assert_eq!(
            msgpack(&a),
            "82a474797065ab757365724f7574636f6d65ac73706c69744f7574636f6d6582a76f7574636f6d65cd2b42a6616d6f756e74a3312e30"
        );
    }

    /// The scale is the caller's: nothing here normalises or pads.
    #[test]
    fn the_amount_keeps_the_scale_it_was_built_with() {
        for (amount, want) in [
            (Decimal::new(10, 1), "1.0"),
            (Decimal::new(1, 0), "1"),
            (Decimal::new(150_000_000, 8), "1.50000000"),
            (Decimal::new(1230, 1), "123.0"),
        ] {
            let v = serde_json::to_value(SplitOutcome::new(1, amount)).unwrap();
            assert_eq!(v["amount"], want);
        }
    }

    /// `null` is the maximal merge, so its encoding is pinned: an absent key or a `0` would be a
    /// different request.
    #[test]
    fn a_merge_of_everything_sends_a_null_amount() {
        assert_eq!(
            serde_json::to_string(&MergeOutcome::all(11074)).unwrap(),
            r#"{"outcome":11074,"amount":null}"#
        );
        assert_eq!(
            serde_json::to_string(&MergeQuestion::all(1283)).unwrap(),
            r#"{"question":1283,"amount":null}"#
        );
        assert_eq!(
            serde_json::to_string(&MergeOutcome::new(7, Decimal::new(5, 1))).unwrap(),
            r#"{"outcome":7,"amount":"0.5"}"#
        );
    }

    #[test]
    fn negate_names_the_question_then_the_outcome() {
        assert_eq!(
            serde_json::to_string(&NegateOutcome::new(1283, 17209, Decimal::new(10, 1))).unwrap(),
            r#"{"question":1283,"outcome":17209,"amount":"1.0"}"#
        );
    }

    /// The nonce is attached by the signing path and must never reach the wire.
    #[test]
    fn the_nonce_is_not_serialized() {
        let mut a = SplitOutcome::new(1, Decimal::ONE);
        a.nonce = Some(1_700_000_000_000);
        assert!(!serde_json::to_string(&a).unwrap().contains("nonce"));
        let mut m = MergeOutcome::all(1);
        m.nonce = Some(1_700_000_000_000);
        assert!(!serde_json::to_string(&m).unwrap().contains("nonce"));
    }
}
