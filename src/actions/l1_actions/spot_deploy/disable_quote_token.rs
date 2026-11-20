use hl_rs_derive::L1Action;
use serde::{Deserialize, Serialize};

/// Disable a token as a quote token. The inverse of
/// [`EnableQuoteToken`](super::EnableQuoteToken), in the docs' `SpotDeployAction` union.
#[derive(Serialize, Deserialize, Debug, Clone, L1Action)]
#[action(action_type = "spotDeploy", payload_key = "disableQuoteToken")]
#[serde(rename_all = "camelCase")]
pub struct DisableQuoteToken {
    pub token: u32,
    #[serde(skip_serializing)]
    pub nonce: Option<u64>,
}

impl DisableQuoteToken {
    pub fn new(token: u32) -> Self {
        Self { token, nonce: None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::L1ActionWrapper;

    /// The docs shape, which recovered its signer (testnet 2026-11-20: signed by a key approved
    /// nowhere, the answer named that key); against CPython `msgpack.packb`.
    #[test]
    fn msgpack_matches_python_reference() {
        let a = DisableQuoteToken::new(2);
        assert_eq!(
            alloy::hex::encode(rmp_serde::to_vec_named(&L1ActionWrapper { action: &a }).unwrap()),
            "82a474797065aa73706f744465706c6f79b164697361626c6551756f7465546f6b656e81a5746f6b656e02"
        );
    }
}
