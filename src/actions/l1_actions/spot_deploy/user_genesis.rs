use hl_rs_derive::L1Action;
use serde::{Deserialize, Serialize};

use crate::actions::serialization::ser_lowercase_user_flags;

/// User genesis allocation for token distribution.
#[derive(Serialize, Deserialize, Debug, Clone, L1Action)]
#[action(action_type = "spotDeploy", payload_key = "userGenesis")]
#[serde(rename_all = "camelCase")]
pub struct UserGenesis {
    /// Token index
    pub token: u32,
    /// User addresses and their wei allocations
    pub user_and_wei: Vec<(String, String)>,
    /// Existing token IDs and their wei allocations
    pub existing_token_and_wei: Vec<(u32, String)>,
    /// `[user, blacklist]` pairs: `true` blacklists the user from the genesis, `false` removes them
    /// from the blacklist. The docs type is `Option<Vec<[String, bool]>>`.
    ///
    /// Skipped when `None`, so a genesis without it signs exactly the SDK's dict, which has no such
    /// key. Each address is sent lowercase, as the SDK sends `userAndWei`, so a checksummed address
    /// signs the same body. The absent key and `[[addr,true]]` recover the signer; `null` does not
    /// (testnet 2026-11-20: signed by a key approved nowhere, the answer named that key).
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "ser_lowercase_user_flags"
    )]
    pub blacklist_users: Option<Vec<(String, bool)>>,
    #[serde(skip_serializing)]
    pub nonce: Option<u64>,
}

impl UserGenesis {
    pub fn new(
        token: u32,
        user_and_wei: Vec<(String, String)>,
        existing_token_and_wei: Vec<(u32, String)>,
    ) -> Self {
        Self {
            token,
            user_and_wei,
            existing_token_and_wei,
            blacklist_users: None,
            nonce: None,
        }
    }

    /// Add or remove users from the genesis blacklist (`true` adds, `false` removes).
    pub fn with_blacklist_users(mut self, blacklist_users: Vec<(String, bool)>) -> Self {
        self.blacklist_users = Some(blacklist_users);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::L1ActionWrapper;

    const A: &str = "0x1111111111111111111111111111111111111111";
    const B: &str = "0x2222222222222222222222222222222222222222";

    fn msgpack_hex(a: &UserGenesis) -> String {
        alloy::hex::encode(rmp_serde::to_vec_named(&L1ActionWrapper { action: a }).unwrap())
    }

    fn base() -> UserGenesis {
        UserGenesis::new(2, vec![(A.into(), "100".into())], vec![(1, "5".into())])
    }

    /// `Exchange.spot_deploy_user_genesis(2, [(A, "100")], [(1, "5")])` in SDK 0.21.0, through
    /// CPython `msgpack.packb`. Tuples pack as arrays on both sides.
    #[test]
    fn without_a_blacklist_it_is_the_sdk_dict() {
        let a = base();
        assert!(!serde_json::to_string(&a).unwrap().contains("blacklist"));
        assert_eq!(
            msgpack_hex(&a),
            "82a474797065aa73706f744465706c6f79ab7573657247656e6573697383a5746f6b656e02aa75736572416e645765699192d92a307831313131313131313131313131313131313131313131313131313131313131313131313131313131a3313030b36578697374696e67546f6b656e416e64576569919201a135"
        );
    }

    /// The docs' `blacklistUsers`, last, as `[user, bool]` arrays: the shape that recovered its
    /// signer (testnet 2026-11-20: signed by a key approved nowhere, the answer named that key).
    #[test]
    fn the_blacklist_is_last_as_user_bool_pairs() {
        let a = base().with_blacklist_users(vec![(B.into(), true), (A.into(), false)]);
        let json = serde_json::to_string(&a).unwrap();
        assert!(json.ends_with(&format!(
            r#""blacklistUsers":[["{B}",true],["{A}",false]]}}"#
        )));
        assert_eq!(
            msgpack_hex(&a),
            "82a474797065aa73706f744465706c6f79ab7573657247656e6573697384a5746f6b656e02aa75736572416e645765699192d92a307831313131313131313131313131313131313131313131313131313131313131313131313131313131a3313030b36578697374696e67546f6b656e416e64576569919201a135ae626c61636b6c69737455736572739292d92a307832323232323232323232323232323232323232323232323232323232323232323232323232323232c392d92a307831313131313131313131313131313131313131313131313131313131313131313131313131313131c2"
        );
    }

    /// A checksummed blacklist address signs the lowercase body.
    #[test]
    fn a_checksummed_blacklist_address_is_sent_lowercase() {
        let checksummed = "0xAaF00000000000000000000000000000000000aB";
        let a = base().with_blacklist_users(vec![(checksummed.into(), true)]);
        let lower = base().with_blacklist_users(vec![(checksummed.to_lowercase(), true)]);
        assert_eq!(msgpack_hex(&a), msgpack_hex(&lower));
        assert!(serde_json::to_string(&a)
            .unwrap()
            .contains(&checksummed.to_lowercase()));
    }
}
