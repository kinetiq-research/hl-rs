use hl_rs_derive::L1Action;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::actions::serialization::serialize_opt_decimal_wire;

/// Which side of a borrow/lend reserve an action moves.
///
/// `Supply` and `Withdraw` are the two HyperCore's CoreWriter precompile also exposes (as
/// `encodedOperation` 0 and 1), and both have been exercised against the live testnet API.
/// `Borrow` and `Repay` exist on the `/exchange` endpoint but have **not** been exercised here — the
/// spelling comes from the API reference rather than from a request that was accepted. Note that
/// third-party references disagree about this field: at least one documents `deposit` where
/// `supply` is correct, so treat the two unverified spellings with the same suspicion.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BorrowLendOperation {
    /// Move tokens from the spot balance into the reserve. Verified.
    Supply,
    /// Move supplied tokens back out of the reserve into the spot balance. Verified.
    Withdraw,
    /// Borrow against supplied collateral. **Spelling unverified.**
    Borrow,
    /// Repay borrowed tokens. **Spelling unverified.**
    Repay,
}

/// Supply to or withdraw from a HyperCore borrow/lend reserve.
///
/// This is an **L1 action**, so it can be signed by an agent (API wallet) rather than needing the
/// master key. That was measured rather than assumed: an agent-signed `supply` reaches the lending
/// module's own business validation, returning `Borrow/Lend error: Minimum amount supplied is $1`
/// after ~790 ms, where a signing or permission rejection would have come back in ~140 ms without
/// ever reaching the module.
///
/// # Field order is the wire format
///
/// The action is msgpacked *before* it is hashed, so the declaration order of these fields is part
/// of the signature rather than a presentation detail. The order below —
/// `operation`, `token`, `amount` — is what was signed and accepted:
///
/// ```json
/// {"type":"borrowLend","operation":"supply","token":0,"amount":"1"}
/// ```
///
/// Reordering them produces valid-looking signatures that HyperCore rejects, with an error that
/// points at the signer rather than at the encoding.
///
/// # Two sharp edges
///
/// **A `None` amount means *the entire balance*,** not "nothing" and not "unspecified". HyperCore
/// documents the null case as the full amount, and CoreWriter documents its `wei = 0` the same way.
/// [`BorrowLend::withdraw_all`] and [`BorrowLend::repay_all`] exist so that reaching for it is an
/// explicit choice at the call site; prefer them over constructing `amount: None` by hand.
///
/// **There is a minimum supply of $1, and it is exclusive.** Supplies of `0.5` and of exactly `1`
/// were both rejected with `Minimum amount supplied is $1`, so an amount must be strictly greater
/// than a dollar despite a message that reads inclusive.
#[derive(Serialize, Deserialize, Debug, Clone, L1Action)]
#[serde(rename_all = "camelCase")]
pub struct BorrowLend {
    /// Which direction to move tokens.
    pub operation: BorrowLendOperation,
    /// HyperCore spot token index. `0` is USDC.
    pub token: u64,
    /// Amount in human-readable units, not wei.
    ///
    /// **`None` serializes as `null`, which HyperCore reads as the full amount.** See the type-level
    /// docs; use [`BorrowLend::withdraw_all`] rather than writing `None` here.
    ///
    /// Sent normalised (`1.50000000` -> `"1.5"`), as orders are and as the SDK's `float_to_wire`
    /// renders amounts, whatever scale the `Decimal` was built at.
    #[serde(serialize_with = "serialize_opt_decimal_wire")]
    pub amount: Option<Decimal>,
    #[serde(skip_serializing)]
    pub nonce: Option<u64>,
}

impl BorrowLend {
    /// Supply `amount` of `token` into its reserve.
    ///
    /// The amount must be strictly greater than $1; see the type-level docs.
    pub fn supply(token: u64, amount: Decimal) -> Self {
        Self {
            operation: BorrowLendOperation::Supply,
            token,
            amount: Some(amount),
            nonce: None,
        }
    }

    /// Withdraw `amount` of `token` back out of its reserve.
    pub fn withdraw(token: u64, amount: Decimal) -> Self {
        Self {
            operation: BorrowLendOperation::Withdraw,
            token,
            amount: Some(amount),
            nonce: None,
        }
    }

    /// Withdraw the **entire** supplied position in `token`.
    ///
    /// Named rather than reachable by passing `None`, because the null amount is the largest action
    /// this type can express and nothing about the value `None` says so.
    pub fn withdraw_all(token: u64) -> Self {
        Self {
            operation: BorrowLendOperation::Withdraw,
            token,
            amount: None,
            nonce: None,
        }
    }

    /// Borrow `amount` of `token` against supplied collateral.
    ///
    /// The `borrow` spelling is taken from the API reference and has not been exercised against the
    /// live endpoint; see [`BorrowLendOperation`].
    pub fn borrow(token: u64, amount: Decimal) -> Self {
        Self {
            operation: BorrowLendOperation::Borrow,
            token,
            amount: Some(amount),
            nonce: None,
        }
    }

    /// Repay `amount` of borrowed `token`. Spelling unverified, as for [`BorrowLend::borrow`].
    pub fn repay(token: u64, amount: Decimal) -> Self {
        Self {
            operation: BorrowLendOperation::Repay,
            token,
            amount: Some(amount),
            nonce: None,
        }
    }

    /// Repay the **entire** borrowed balance in `token`. Spelling unverified.
    pub fn repay_all(token: u64) -> Self {
        Self {
            operation: BorrowLendOperation::Repay,
            token,
            amount: None,
            nonce: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Action;
    use rust_decimal_macros::dec;

    #[test]
    fn the_action_type_is_borrow_lend() {
        assert_eq!(<BorrowLend as Action>::ACTION_TYPE, "borrowLend");
    }

    /// Pins the wire form that HyperCore accepted, field order included. The action is msgpacked
    /// before hashing, so this ordering is part of the signature — a reordering here would produce
    /// signatures that fail with an error naming the signer rather than the encoding.
    #[test]
    fn the_wire_form_matches_what_hypercore_accepted() {
        let json = serde_json::to_string(&BorrowLend::supply(0, dec!(1.5))).unwrap();
        assert_eq!(
            json, r#"{"operation":"supply","token":0,"amount":"1.5"}"#,
            "field order and the string-encoded amount are both load-bearing"
        );
    }

    /// The null amount is the full balance, so its encoding is worth pinning separately: a change
    /// that made `None` serialize as an absent key, or as `0`, would mean something different.
    /// A `Decimal` built at a fixed scale goes out normalised. Before this, an amount built as
    /// 8-decimal fixed point (`1.50000000`) was sent with its scale, which is a different string
    /// and so a different msgpack and hash from the SDK-style `"1.5"`. The msgpack is checked
    /// against CPython `msgpack.packb` of `{"type":"borrowLend",...,"amount":"1.5"}`.
    #[test]
    fn a_scaled_amount_is_sent_normalised() {
        let a = BorrowLend::supply(0, Decimal::from_i128_with_scale(150_000_000, 8));
        assert_eq!(
            serde_json::to_string(&a).unwrap(),
            r#"{"operation":"supply","token":0,"amount":"1.5"}"#
        );
        assert_eq!(
            alloy::hex::encode(
                rmp_serde::to_vec_named(&crate::actions::L1ActionWrapper { action: &a }).unwrap()
            ),
            "84a474797065aa626f72726f774c656e64a96f7065726174696f6ea6737570706c79a5746f6b656e00a6616d6f756e74a3312e35"
        );
        for (scaled, want) in [
            (Decimal::from_i128_with_scale(100_000_000, 8), "1"),
            (Decimal::from_i128_with_scale(1, 8), "0.00000001"),
            (Decimal::from_i128_with_scale(1_000_000_000_000, 8), "10000"),
        ] {
            let v = serde_json::to_value(BorrowLend::withdraw(0, scaled)).unwrap();
            assert_eq!(v["amount"], want);
        }
    }

    #[test]
    fn withdraw_all_serializes_a_null_amount() {
        let json = serde_json::to_string(&BorrowLend::withdraw_all(0)).unwrap();
        assert_eq!(json, r#"{"operation":"withdraw","token":0,"amount":null}"#);
    }

    #[test]
    fn the_operation_spellings_are_lowercase() {
        for (op, want) in [
            (BorrowLendOperation::Supply, r#""supply""#),
            (BorrowLendOperation::Withdraw, r#""withdraw""#),
            (BorrowLendOperation::Borrow, r#""borrow""#),
            (BorrowLendOperation::Repay, r#""repay""#),
        ] {
            assert_eq!(serde_json::to_string(&op).unwrap(), want);
        }
    }

    /// The nonce is attached by the signing path and must never reach the wire, or it would appear
    /// twice in the hashed payload.
    #[test]
    fn the_nonce_is_not_serialized() {
        let mut action = BorrowLend::supply(0, dec!(2));
        action.nonce = Some(1_793_000_000_000);
        assert!(!serde_json::to_string(&action).unwrap().contains("nonce"));
    }
}
