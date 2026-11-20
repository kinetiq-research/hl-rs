use hl_rs_derive::L1Action;
use serde::{Deserialize, Serialize};

/// Token specification for registration.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TokenSpec {
    /// Token name/symbol
    pub name: String,
    /// Size decimals
    pub sz_decimals: u32,
    /// Wei decimals
    pub wei_decimals: u32,
}

/// Register a new token.
#[derive(Serialize, Deserialize, Debug, Clone, L1Action)]
#[action(action_type = "spotDeploy", payload_key = "registerToken2")]
#[serde(rename_all = "camelCase")]
pub struct RegisterToken {
    /// Token specification
    pub spec: TokenSpec,
    /// Maximum gas for the operation
    pub max_gas: u64,
    /// Full display name for the token.
    ///
    /// Optional in the docs, and skipped when `None`. An `Option` rather than a `String` so that
    /// "no full name" is the absent key and never an empty string signed by accident.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub full_name: Option<String>,
    #[serde(skip_serializing)]
    pub nonce: Option<u64>,
}

impl RegisterToken {
    pub fn new(
        name: impl Into<String>,
        sz_decimals: u32,
        wei_decimals: u32,
        max_gas: u64,
        full_name: impl Into<String>,
    ) -> Self {
        Self {
            spec: TokenSpec {
                name: name.into(),
                sz_decimals,
                wei_decimals,
            },
            max_gas,
            full_name: Some(full_name.into()),
            nonce: None,
        }
    }

    /// As [`RegisterToken::new`], with no `fullName` key on the wire.
    pub fn without_full_name(
        name: impl Into<String>,
        sz_decimals: u32,
        wei_decimals: u32,
        max_gas: u64,
    ) -> Self {
        Self {
            full_name: None,
            ..Self::new(name, sz_decimals, wei_decimals, max_gas, "")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::L1ActionWrapper;

    fn msgpack_hex(a: &RegisterToken) -> String {
        alloy::hex::encode(rmp_serde::to_vec_named(&L1ActionWrapper { action: a }).unwrap())
    }

    /// `Exchange.spot_deploy_register_token("PZQX", 2, 8, 1, "p0 probe")` in SDK 0.21.0, through
    /// CPython `msgpack.packb`. Also the body HyperCore answered on its gas check, for agent and
    /// master alike, on testnet 2026-11-20.
    #[test]
    fn with_a_full_name_it_is_the_sdk_dict() {
        let a = RegisterToken::new("PZQX", 2, 8, 1, "p0 probe");
        assert_eq!(
            msgpack_hex(&a),
            "82a474797065aa73706f744465706c6f79ae7265676973746572546f6b656e3283a47370656383a46e616d65a4505a5158aa737a446563696d616c7302ab776569446563696d616c7308a66d617847617301a866756c6c4e616d65a870302070726f6265"
        );
    }

    /// The docs mark `fullName` optional; omitted, the key is absent rather than `""` or `null`.
    #[test]
    fn without_a_full_name_the_key_is_absent() {
        let a = RegisterToken::without_full_name("PZQX", 2, 8, 1);
        assert!(!serde_json::to_string(&a).unwrap().contains("fullName"));
        assert_eq!(
            msgpack_hex(&a),
            "82a474797065aa73706f744465706c6f79ae7265676973746572546f6b656e3282a47370656383a46e616d65a4505a5158aa737a446563696d616c7302ab776569446563696d616c7308a66d617847617301"
        );
    }
}
