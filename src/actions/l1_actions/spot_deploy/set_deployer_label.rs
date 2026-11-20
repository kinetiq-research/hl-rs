use hl_rs_derive::L1Action;
use serde::{Deserialize, Serialize};

/// Set the deployer's label. Per the docs: 2-4 lowercase characters, settable once per deployer,
/// and only by an account that has deployed a spot token.
///
/// The label is sent as given. Validating it is the caller's job; HyperCore refuses a deployer
/// with no token before it reads the label (measured on testnet, 2026-11-20).
#[derive(Serialize, Deserialize, Debug, Clone, L1Action)]
#[action(action_type = "spotDeploy", payload_key = "setDeployerLabel")]
#[serde(rename_all = "camelCase")]
pub struct SetDeployerLabel {
    pub label: String,
    #[serde(skip_serializing)]
    pub nonce: Option<u64>,
}

impl SetDeployerLabel {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            nonce: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::L1ActionWrapper;

    /// The docs shape, which HyperCore answered with `deployer has not deployed a spot token` (a
    /// recovered signer) on testnet, 2026-11-20; against CPython `msgpack.packb`.
    #[test]
    fn msgpack_matches_python_reference() {
        let a = SetDeployerLabel::new("qzx");
        assert_eq!(
            alloy::hex::encode(rmp_serde::to_vec_named(&L1ActionWrapper { action: &a }).unwrap()),
            "82a474797065aa73706f744465706c6f79b07365744465706c6f7965724c6162656c81a56c6162656ca3717a78"
        );
    }
}
