use hl_rs_derive::L1Action;
use serde::{Deserialize, Serialize};

use crate::actions::serialization::is_false;

/// Genesis configuration for a token.
#[derive(Serialize, Deserialize, Debug, Clone, L1Action)]
#[action(action_type = "spotDeploy", payload_key = "genesis")]
#[serde(rename_all = "camelCase")]
pub struct Genesis {
    /// Token index
    pub token: u32,
    /// Maximum supply
    pub max_supply: String,
    /// Whether to disable hyperliquidity.
    ///
    /// **Sent only when `true`.** The Python SDK adds `"noHyperliquidity": true` to the dict only
    /// in that case, and the docs mark the key optional. `false` is the absent key, never a `false`
    /// value. Measured (testnet 2026-11-20: signed by a key approved nowhere, the answer named that
    /// key): `true` and the absent key recover the signer; `"noHyperliquidity":false` recovers a
    /// random address.
    #[serde(default, skip_serializing_if = "is_false")]
    pub no_hyperliquidity: bool,
    #[serde(skip_serializing)]
    pub nonce: Option<u64>,
}

impl Genesis {
    pub fn new(token: u32, max_supply: impl Into<String>, no_hyperliquidity: bool) -> Self {
        Self {
            token,
            max_supply: max_supply.into(),
            no_hyperliquidity,
            nonce: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::L1ActionWrapper;

    fn msgpack_hex(a: &Genesis) -> String {
        alloy::hex::encode(rmp_serde::to_vec_named(&L1ActionWrapper { action: a }).unwrap())
    }

    /// `Exchange.spot_deploy_genesis(2, "1", False)` in SDK 0.21.0, through CPython
    /// `msgpack.packb`: no `noHyperliquidity` key at all. Also a body that recovered its signer
    /// (testnet 2026-11-20: signed by a key approved nowhere, the answer named that key).
    #[test]
    fn false_omits_the_key() {
        let a = Genesis::new(2, "1", false);
        assert_eq!(
            serde_json::to_string(&a).unwrap(),
            r#"{"token":2,"maxSupply":"1"}"#
        );
        assert_eq!(
            msgpack_hex(&a),
            "82a474797065aa73706f744465706c6f79a767656e6573697382a5746f6b656e02a96d6178537570706c79a131"
        );
    }

    /// `Exchange.spot_deploy_genesis(2, "1", True)`: the key, last, as `true`.
    #[test]
    fn true_sends_the_key_last() {
        let a = Genesis::new(2, "1", true);
        assert_eq!(
            serde_json::to_string(&a).unwrap(),
            r#"{"token":2,"maxSupply":"1","noHyperliquidity":true}"#
        );
        assert_eq!(
            msgpack_hex(&a),
            "82a474797065aa73706f744465706c6f79a767656e6573697383a5746f6b656e02a96d6178537570706c79a131b06e6f48797065726c6971756964697479c3"
        );
    }

    #[test]
    fn an_absent_key_deserialises_as_false() {
        let a: Genesis = serde_json::from_str(r#"{"token":2,"maxSupply":"1"}"#).unwrap();
        assert!(!a.no_hyperliquidity);
    }
}
