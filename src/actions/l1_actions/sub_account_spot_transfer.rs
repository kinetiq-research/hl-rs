use alloy::primitives::Address;
use hl_rs_derive::L1Action;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::actions::serialization::{ser_lowercase, serialize_decimal_wire};

/// Transfer spot tokens between main account and sub-account.
#[derive(Serialize, Deserialize, Debug, Clone, L1Action)]
#[serde(rename_all = "camelCase")]
pub struct SubAccountSpotTransfer {
    /// Sub-account user address
    #[serde(serialize_with = "ser_lowercase")]
    pub sub_account_user: Address,
    /// True to deposit to sub-account, false to withdraw
    pub is_deposit: bool,
    /// Token identifier
    pub token: String,
    /// Amount to transfer, sent normalised as orders are (`1.50` -> `"1.5"`).
    #[serde(serialize_with = "serialize_decimal_wire")]
    pub amount: Decimal,
    #[serde(skip_serializing)]
    pub nonce: Option<u64>,
}

impl SubAccountSpotTransfer {
    /// Deposit tokens to a sub-account
    pub fn deposit(sub_account_user: Address, token: impl Into<String>, amount: Decimal) -> Self {
        Self {
            sub_account_user,
            is_deposit: true,
            token: token.into(),
            amount,
            nonce: None,
        }
    }

    /// Withdraw tokens from a sub-account
    pub fn withdraw(sub_account_user: Address, token: impl Into<String>, amount: Decimal) -> Self {
        Self {
            sub_account_user,
            is_deposit: false,
            token: token.into(),
            amount,
            nonce: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::L1ActionWrapper;
    use rust_decimal_macros::dec;

    /// The amount goes out normalised, in the SDK's field order, against CPython `msgpack.packb`
    /// of `Exchange.sub_account_spot_transfer`'s dict with `"amount": "1.5"`.
    #[test]
    fn the_amount_is_sent_normalised() {
        let a = SubAccountSpotTransfer::deposit(Address::repeat_byte(0x11), "USDC", dec!(1.50));
        assert_eq!(
            alloy::hex::encode(rmp_serde::to_vec_named(&L1ActionWrapper { action: &a }).unwrap()),
            "85a474797065b67375624163636f756e7453706f745472616e73666572ae7375624163636f756e7455736572d92a307831313131313131313131313131313131313131313131313131313131313131313131313131313131a969734465706f736974c3a5746f6b656ea455534443a6616d6f756e74a3312e35"
        );
    }
}
