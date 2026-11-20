use alloy::primitives::Address;
use hl_rs_derive::L1Action;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::actions::serialization::{ser_lowercase, serialize_decimal_wire};
use crate::actions::DexId;

/// Send assets between DEXes **on behalf of the principal, signed by an agent**.
///
/// # Why this exists separately from [`crate::actions::SendAsset`]
///
/// `sendAsset` is a user-signed EIP-712 action: its typed data carries `hyperliquidChain` and
/// `signatureChainId` and has no source-account field, so the source *is* the recovering address.
/// An agent signing it acts as itself.
///
/// `agentSendAsset` is the L1 variant of the same operation. HyperLiquid's exchange-endpoint
/// documentation describes it as *"Similar to send asset, but can be signed by an agent. Destination
/// must match the source address."* Being an L1 action, it is msgpack-hashed and signed through the
/// phantom-agent construct, so the signature resolves to the agent's **master** — which is exactly
/// what makes it agent-usable.
///
/// # What it means for an agent's authority
///
/// `sourceDex`/`destinationDex` take `""` for the default USDC perp DEX and `"spot"` for spot, so
/// this performs a **perp <-> spot class transfer** — the same move as the user-signed
/// `usdClassTransfer`. An agent therefore *can* relocate the master's collateral between venues.
///
/// The counterparty constraint is what bounds it: destination must equal the source, so it cannot
/// send value to a third party. That distinction — an agent may **relocate** value inside the
/// account's own footprint but may not **send it away** — is the precise form of the claim, and it
/// is narrower than "an agent cannot move value".
#[derive(Serialize, Deserialize, Debug, Clone, L1Action)]
#[serde(rename_all = "camelCase")]
pub struct AgentSendAsset {
    /// Destination address. Per HyperLiquid's docs this must match the source address.
    #[serde(serialize_with = "ser_lowercase")]
    pub destination: Address,
    /// Source DEX: `""` for the default USDC perp DEX, `"spot"` for spot.
    pub source_dex: DexId,
    /// Destination DEX, same encoding.
    pub destination_dex: DexId,
    /// Token to move, in HyperLiquid's `tokenName:tokenId` form -- e.g.
    /// `USDC:0xeb62eee3685fc4c43992febcd9e75443` on testnet, from `/info` `spotMeta`. A bare
    /// `"USDC"` is a 422. Only the collateral token may cross a perp DEX boundary.
    pub token: String,
    /// Amount to move, as a normalised string. A JSON number is refused with a bare 422,
    /// `"Failed to deserialize the JSON body into the target type"` (measured 2026-11-16).
    #[serde(serialize_with = "serialize_decimal_wire")]
    pub amount: Decimal,
    /// Source sub-account, or the empty string for the main account.
    ///
    /// **Whose balance this names is not settled by HyperLiquid's documentation.** If "the source"
    /// in the destination constraint means the sub-account, this can only move a sub-account's funds
    /// within that sub-account; if it means the signing master, an agent could pull a sub-account's
    /// funds to the master. Those are materially different authority surfaces. Do not rely on either
    /// reading until it is measured.
    pub from_sub_account: String,
    /// **Serialized, unlike most L1 actions.** HyperLiquid documents the nonce inside this action's
    /// body -- *"current timestamp in milliseconds as a Number, should match nonce"* -- where an
    /// ordinary L1 action carries it only in the outer envelope. Skipping it is a 422.
    ///
    /// The client fills this in when it stamps the envelope nonce, so leave it `None`.
    pub nonce: Option<u64>,
}

impl AgentSendAsset {
    /// Move `amount` of `token` between two of the signer's own venues.
    pub fn between(
        own_address: Address,
        source_dex: DexId,
        destination_dex: DexId,
        token: impl Into<String>,
        amount: Decimal,
    ) -> Self {
        Self {
            destination: own_address,
            source_dex,
            destination_dex,
            token: token.into(),
            amount,
            from_sub_account: String::new(),
            nonce: None,
        }
    }

    /// A perp -> spot class transfer of the collateral token, the agent-signable equivalent of
    /// `usdClassTransfer`.
    pub fn perp_to_spot(own_address: Address, token: impl Into<String>, amount: Decimal) -> Self {
        Self::between(own_address, DexId::Perp, DexId::Spot, token, amount)
    }

    /// A spot -> perp class transfer of the collateral token.
    pub fn spot_to_perp(own_address: Address, token: impl Into<String>, amount: Decimal) -> Self {
        Self::between(own_address, DexId::Spot, DexId::Perp, token, amount)
    }

    pub fn from_sub_account(mut self, sub_account: impl Into<String>) -> Self {
        self.from_sub_account = sub_account.into();
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::{Action, L1ActionWrapper};
    use rust_decimal_macros::dec;

    fn own() -> Address {
        Address::repeat_byte(0x11)
    }

    fn action() -> AgentSendAsset {
        AgentSendAsset::perp_to_spot(own(), "USDC:0xeb62eee3685fc4c43992febcd9e75443", dec!(1.50))
            .with_nonce(1_795_182_479_639)
    }

    #[test]
    fn the_action_type_is_agent_send_asset() {
        assert_eq!(<AgentSendAsset as Action>::ACTION_TYPE, "agentSendAsset");
    }

    /// The field order that recovered its signer (testnet 2026-11-20: signed by a key approved
    /// nowhere, the answer named that key), with the docs' key names: `destination, sourceDex,
    /// destinationDex, token, amount, fromSubAccount, nonce`.
    #[test]
    fn the_wire_form_matches_the_accepted_body() {
        assert_eq!(
            serde_json::to_string(&action()).unwrap(),
            r#"{"destination":"0x1111111111111111111111111111111111111111","sourceDex":"","destinationDex":"spot","token":"USDC:0xeb62eee3685fc4c43992febcd9e75443","amount":"1.5","fromSubAccount":"","nonce":1795182479639}"#
        );
    }

    /// The hashed bytes, against CPython `msgpack.packb` of the same dict with `"type"` first.
    #[test]
    fn msgpack_matches_python_reference() {
        let a = action();
        let got = rmp_serde::to_vec_named(&L1ActionWrapper { action: &a }).unwrap();
        assert_eq!(
            alloy::hex::encode(got),
            "88a474797065ae6167656e7453656e644173736574ab64657374696e6174696f6ed92a307831313131313131313131313131313131313131313131313131313131313131313131313131313131a9736f75726365446578a0ae64657374696e6174696f6e446578a473706f74a5746f6b656ed927555344433a30786562363265656533363835666334633433393932666562636439653735343433a6616d6f756e74a3312e35ae66726f6d5375624163636f756e74a0a56e6f6e6365cf000001a1f936c917"
        );
    }

    /// The body nonce is the envelope nonce: `with_nonce` stamps both from one value.
    #[test]
    fn the_body_nonce_is_the_envelope_nonce() {
        let a = AgentSendAsset::spot_to_perp(own(), "USDC:0x0", dec!(1)).with_nonce(42);
        assert_eq!(a.nonce(), Some(42));
        assert_eq!(serde_json::to_value(&a).unwrap()["nonce"], 42);
    }
}
