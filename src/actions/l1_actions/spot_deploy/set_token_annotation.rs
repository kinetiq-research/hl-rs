use hl_rs_derive::L1Action;
use serde::{Deserialize, Serialize};

/// A token's annotation, as the docs' `setTokenAnnotation` carries it.
///
/// The docs bound each field (category 15, description 400, display name 9, two keywords of 10);
/// the bounds are the caller's to enforce. Field order is the wire order.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TokenAnnotation {
    pub category: String,
    pub description: String,
    /// **`None` is sent as `null`, not omitted.** That is the shape HyperCore deserialised and
    /// verified (a recovered signer) on testnet, 2026-11-20. The omitted form was not measured.
    pub display_name: Option<String>,
    pub keywords: Vec<String>,
}

/// Set a token's annotation. Per the docs, at most once per day.
#[derive(Serialize, Deserialize, Debug, Clone, L1Action)]
#[action(action_type = "spotDeploy", payload_key = "setTokenAnnotation")]
#[serde(rename_all = "camelCase")]
pub struct SetTokenAnnotation {
    pub token: u32,
    pub annotation: TokenAnnotation,
    #[serde(skip_serializing)]
    pub nonce: Option<u64>,
}

impl SetTokenAnnotation {
    pub fn new(token: u32, annotation: TokenAnnotation) -> Self {
        Self {
            token,
            annotation,
            nonce: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::L1ActionWrapper;

    fn msgpack_hex(a: &SetTokenAnnotation) -> String {
        alloy::hex::encode(rmp_serde::to_vec_named(&L1ActionWrapper { action: a }).unwrap())
    }

    /// The body HyperCore answered with `Incorrect deployer` (a recovered signer) on testnet,
    /// 2026-11-20, against CPython `msgpack.packb`: a null display name and no keywords.
    #[test]
    fn the_minimal_annotation_matches_the_measured_body() {
        let a = SetTokenAnnotation::new(
            2,
            TokenAnnotation {
                category: "test".into(),
                description: "probe".into(),
                display_name: None,
                keywords: vec![],
            },
        );
        assert_eq!(
            serde_json::to_string(&a).unwrap(),
            r#"{"token":2,"annotation":{"category":"test","description":"probe","displayName":null,"keywords":[]}}"#
        );
        assert_eq!(
            msgpack_hex(&a),
            "82a474797065aa73706f744465706c6f79b2736574546f6b656e416e6e6f746174696f6e82a5746f6b656e02aa616e6e6f746174696f6e84a863617465676f7279a474657374ab6465736372697074696f6ea570726f6265ab646973706c61794e616d65c0a86b6579776f72647390"
        );
    }

    /// Registered in `ActionKind`, so a signed one dispatches back to this type.
    #[test]
    fn round_trips_through_signed_action_kind() {
        use crate::actions::{ActionKind, PreparedAction, SignedActionKind};
        use crate::SigningChain;
        use alloy::primitives::U256;
        use alloy_signer::Signature;

        let a = SetTokenAnnotation::new(
            7,
            TokenAnnotation {
                category: "c".into(),
                description: "d".into(),
                display_name: None,
                keywords: vec!["k".into()],
            },
        );
        let signed = PreparedAction::new(a, &SigningChain::Testnet, None, None)
            .unwrap()
            .with_signature(Signature::new(U256::from(1), U256::from(2), false));
        let json = serde_json::to_string(&signed).unwrap();
        match SignedActionKind::from_json(&json).unwrap().action {
            ActionKind::SetTokenAnnotation(a) => {
                assert_eq!(a.token, 7);
                assert_eq!(a.annotation.keywords, vec!["k".to_string()]);
            }
            other => panic!("expected SetTokenAnnotation, got {other:?}"),
        }
    }

    /// Every field set, in the same order.
    #[test]
    fn a_full_annotation_keeps_the_order() {
        let a = SetTokenAnnotation::new(
            2,
            TokenAnnotation {
                category: "defi".into(),
                description: "a token".into(),
                display_name: Some("Token".into()),
                keywords: vec!["one".into(), "two".into()],
            },
        );
        assert_eq!(
            msgpack_hex(&a),
            "82a474797065aa73706f744465706c6f79b2736574546f6b656e416e6e6f746174696f6e82a5746f6b656e02aa616e6e6f746174696f6e84a863617465676f7279a464656669ab6465736372697074696f6ea76120746f6b656eab646973706c61794e616d65a5546f6b656ea86b6579776f72647392a36f6e65a374776f"
        );
    }
}
