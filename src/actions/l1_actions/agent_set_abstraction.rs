use hl_rs_derive::L1Action;
use serde::{Deserialize, Serialize};

use crate::actions::AbstractionMode;

/// Account abstraction mode for [`AgentSetAbstraction`], in the one-letter form the
/// agent action takes on the wire.
///
/// The same three modes [`UserSetAbstraction`](crate::UserSetAbstraction) spells out in
/// full; `dexAbstraction` has no agent form. See [account abstraction
/// modes](https://hyperliquid.gitbook.io/hyperliquid-docs/trading/account-abstraction-modes).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AgentAbstractionMode {
    /// `"i"`: abstraction disabled.
    #[serde(rename = "i")]
    Disabled,
    /// `"u"`: unified account.
    #[serde(rename = "u")]
    UnifiedAccount,
    /// `"p"`: portfolio margin.
    #[serde(rename = "p")]
    PortfolioMargin,
}

impl AgentAbstractionMode {
    /// The wire spelling: `"i"`, `"u"` or `"p"`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Disabled => "i",
            Self::UnifiedAccount => "u",
            Self::PortfolioMargin => "p",
        }
    }
}

impl From<AgentAbstractionMode> for AbstractionMode {
    fn from(mode: AgentAbstractionMode) -> Self {
        match mode {
            AgentAbstractionMode::Disabled => AbstractionMode::Disabled,
            AgentAbstractionMode::UnifiedAccount => AbstractionMode::UnifiedAccount,
            AgentAbstractionMode::PortfolioMargin => AbstractionMode::PortfolioMargin,
        }
    }
}

/// Set the account abstraction mode, signed by an agent (API wallet).
///
/// The L1 twin of [`UserSetAbstraction`](crate::UserSetAbstraction): it carries no user
/// and no chain fields, and applies to the account the agent is approved for.
///
/// ```json
/// {"type":"agentSetAbstraction","abstraction":"u"}
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, L1Action)]
pub struct AgentSetAbstraction {
    /// Abstraction mode to set.
    pub abstraction: AgentAbstractionMode,
    #[serde(skip_serializing)]
    pub nonce: Option<u64>,
}

impl AgentSetAbstraction {
    pub fn new(abstraction: AgentAbstractionMode) -> Self {
        Self {
            abstraction,
            nonce: None,
        }
    }

    pub fn disable() -> Self {
        Self::new(AgentAbstractionMode::Disabled)
    }

    pub fn unified_account() -> Self {
        Self::new(AgentAbstractionMode::UnifiedAccount)
    }

    pub fn portfolio_margin() -> Self {
        Self::new(AgentAbstractionMode::PortfolioMargin)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::{Action, L1ActionWrapper, SigningMeta};
    use crate::SigningChain;

    /// Vectors from hyperliquid-python-sdk `2fdb18f`: the dict built by the SDK's own
    /// `Exchange.agent_set_abstraction` at nonce 1700000000000, its `msgpack.packb`, and
    /// the `Agent` digest from `action_hash`, `construct_phantom_agent` and `l1_payload`
    /// (vault and expiry unset). `tests/vectors/gen_agent_set_abstraction.py` writes them.
    const VECTORS: [(AgentAbstractionMode, &str, &str, &str); 3] = [
        (
            AgentAbstractionMode::Disabled,
            "82a474797065b36167656e745365744162737472616374696f6eab6162737472616374696f6ea169",
            "0xbc0ddb62b64b1415f18c1baef48fc1f2296a990368bf913dd36e543f764d9238",
            "0x01a7c531c1ca8b0834b50fa36d8445821bfa1526ab822addba1217427e5b34dd",
        ),
        (
            AgentAbstractionMode::UnifiedAccount,
            "82a474797065b36167656e745365744162737472616374696f6eab6162737472616374696f6ea175",
            "0xab4dbeb3e9ac11dfe71b3810f89d5a8672b79e576063ab04b01b19c328aff32a",
            "0xaef7f282986c70b12367a0faf21650db5e2dc4da4bf39b0013fa401dc993376f",
        ),
        (
            AgentAbstractionMode::PortfolioMargin,
            "82a474797065b36167656e745365744162737472616374696f6eab6162737472616374696f6ea170",
            "0xdb3417cbe90b87372907f6aa6d686e63f409f875999e77e80196de621542b323",
            "0x0a933eb95ba58c7dd8492124921c1651d5371575c5659bb7ba13344ba758ebe5",
        ),
    ];

    fn digest(action: &AgentSetAbstraction, chain: &SigningChain) -> String {
        let meta = SigningMeta {
            nonce: 1_700_000_000_000,
            vault_address: None,
            expires_after: None,
            signing_chain: chain,
        };
        format!("{:#x}", action.signing_hash(&meta).unwrap())
    }

    #[test]
    fn the_action_type_is_agent_set_abstraction() {
        assert_eq!(
            <AgentSetAbstraction as Action>::ACTION_TYPE,
            "agentSetAbstraction"
        );
    }

    #[test]
    fn the_msgpack_and_digests_match_the_sdk() {
        for (mode, msgpack, mainnet, testnet) in VECTORS {
            let action = AgentSetAbstraction::new(mode);
            let packed = rmp_serde::to_vec_named(&L1ActionWrapper { action: &action }).unwrap();
            assert_eq!(alloy::hex::encode(packed), msgpack, "{mode:?} msgpack");
            assert_eq!(digest(&action, &SigningChain::Mainnet), mainnet, "{mode:?}");
            assert_eq!(digest(&action, &SigningChain::Testnet), testnet, "{mode:?}");
        }
    }

    #[test]
    fn the_modes_are_one_letter_on_the_wire() {
        for (action, want) in [
            (AgentSetAbstraction::disable(), r#"{"abstraction":"i"}"#),
            (
                AgentSetAbstraction::unified_account(),
                r#"{"abstraction":"u"}"#,
            ),
            (
                AgentSetAbstraction::portfolio_margin(),
                r#"{"abstraction":"p"}"#,
            ),
        ] {
            assert_eq!(serde_json::to_string(&action).unwrap(), want);
            assert_eq!(
                format!(r#"{{"abstraction":"{}"}}"#, action.abstraction.as_str()),
                want
            );
        }
    }

    #[test]
    fn each_mode_names_its_user_signed_counterpart() {
        for (mode, want) in [
            (AgentAbstractionMode::Disabled, "disabled"),
            (AgentAbstractionMode::UnifiedAccount, "unifiedAccount"),
            (AgentAbstractionMode::PortfolioMargin, "portfolioMargin"),
        ] {
            assert_eq!(AbstractionMode::from(mode).as_str(), want);
        }
    }

    /// The SDK dict reads back as this action, so a signed body can be re-verified.
    #[test]
    fn the_sdk_dict_parses_as_this_action() {
        let body = r#"{"action":{"type":"agentSetAbstraction","abstraction":"p"},
            "nonce":1700000000000,"signature":{"r":"0x1","s":"0x2","v":27}}"#;
        let signed = crate::actions::SignedActionKind::from_json(body).unwrap();
        match signed.action {
            crate::actions::ActionKind::AgentSetAbstraction(a) => {
                assert_eq!(a.abstraction, AgentAbstractionMode::PortfolioMargin)
            }
            other => panic!("parsed as {other:?}"),
        }
    }

    /// The nonce is attached by the signing path and must never reach the wire.
    #[test]
    fn the_nonce_is_not_serialized() {
        let mut action = AgentSetAbstraction::unified_account();
        action.nonce = Some(1_700_000_000_000);
        assert!(!serde_json::to_string(&action).unwrap().contains("nonce"));
    }
}
