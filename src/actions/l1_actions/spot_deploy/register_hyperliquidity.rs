use hl_rs_derive::L1Action;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::actions::serialization::serialize_decimal_wire;

/// Register hyperliquidity for a spot market.
#[derive(Serialize, Deserialize, Debug, Clone, L1Action)]
#[action(action_type = "spotDeploy", payload_key = "registerHyperliquidity")]
#[serde(rename_all = "camelCase")]
pub struct RegisterHyperliquidity {
    /// Spot market index
    pub spot: u32,
    /// Starting price, sent normalised as orders are (`2.0` -> `"2"`).
    #[serde(serialize_with = "serialize_decimal_wire")]
    pub start_px: Decimal,
    /// Order size, sent normalised as `start_px`.
    #[serde(serialize_with = "serialize_decimal_wire")]
    pub order_sz: Decimal,
    /// Number of orders
    pub n_orders: u32,
    /// Number of seeded price levels (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n_seeded_levels: Option<u32>,
    #[serde(skip_serializing)]
    pub nonce: Option<u64>,
}

impl RegisterHyperliquidity {
    pub fn new(spot: u32, start_px: Decimal, order_sz: Decimal, n_orders: u32) -> Self {
        Self {
            spot,
            start_px,
            order_sz,
            n_orders,
            n_seeded_levels: None,
            nonce: None,
        }
    }

    pub fn with_seeded_levels(mut self, n_seeded_levels: u32) -> Self {
        self.n_seeded_levels = Some(n_seeded_levels);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::L1ActionWrapper;
    use rust_decimal_macros::dec;

    /// Prices and sizes go out normalised, against CPython `msgpack.packb` of
    /// `{"spot":1,"startPx":"2","orderSz":"1.25","nOrders":3}`. Both `"2.0"` and `"2"` reached the
    /// deployer check on testnet (2026-11-20), so HyperCore hashes either as sent; this picks the
    /// one rendering orders use. Note the SDK's own method sends `str(float)`, i.e. `"2.0"`.
    #[test]
    fn decimals_are_sent_normalised() {
        let a = RegisterHyperliquidity::new(1, dec!(2.0), dec!(1.2500), 3);
        assert_eq!(
            serde_json::to_string(&a).unwrap(),
            r#"{"spot":1,"startPx":"2","orderSz":"1.25","nOrders":3}"#
        );
        assert_eq!(
            alloy::hex::encode(rmp_serde::to_vec_named(&L1ActionWrapper { action: &a }).unwrap()),
            "82a474797065aa73706f744465706c6f79b6726567697374657248797065726c697175696469747984a473706f7401a773746172745078a132a76f72646572537aa4312e3235a76e4f726465727303"
        );
    }
}
