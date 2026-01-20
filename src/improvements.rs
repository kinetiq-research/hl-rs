use crate::SigningChain;
use alloy::{
    dyn_abi::Eip712Domain, primitives::{Address, B256, address, keccak256}, signers::local::PrivateKeySigner, sol_types::{SolValue, eip712_domain}
};
use alloy_signer::{Error as SignerError, Signature, SignerSync};
use serde::{Deserialize, Serialize, Serializer};

pub struct SigningMeta {
    pub signing_chain: SigningChain,
    pub nonce: u64,
    pub vault_address: Option<Address>,
    pub expires_after: Option<u64>,
}

pub trait Action {
    fn signing_hash(&self, signing_meta: &SigningMeta) -> B256;

    fn sign(
        &self,
        wallet: &PrivateKeySigner,
        signing_meta: &SigningMeta,
    ) -> Result<Signature, SignerError> {
        wallet.sign_hash_sync(&self.signing_hash(signing_meta))
    }
}

pub trait Eip712 {
    fn domain(&self, signing_meta: &SigningMeta) -> Eip712Domain;

    fn struct_hash(&self, signing_meta: &SigningMeta) -> B256;

    fn eip712_signing_hash(&self, signing_meta: &SigningMeta) -> B256 {
        let mut digest_input = [0u8; 2 + 32 + 32];
        digest_input[0] = 0x19;
        digest_input[1] = 0x01;
        digest_input[2..34].copy_from_slice(&self.domain(signing_meta).hash_struct()[..]);
        digest_input[34..66].copy_from_slice(&self.struct_hash(signing_meta)[..]);

        keccak256(digest_input)
    }
}

fn eip_712_domain() -> Eip712Domain {
    eip712_domain! {
        name: "HyperliquidSignTransaction",
        version: "1",
        chain_id: 0x66eee,
        verifying_contract: Address::ZERO,
    }
}

impl Eip712 for UsdSend {
    fn domain(&self, signing_meta: &SigningMeta) -> Eip712Domain {
        eip_712_domain()
    }

    fn struct_hash(&self, signing_meta: &SigningMeta) -> B256 {
        let items = (
            keccak256(
                "HyperliquidTransaction:UsdSend(string hyperliquidChain,string destination,string amount,uint64 time)",
            ),
            keccak256(&signing_meta.signing_chain.get_hyperliquid_chain()),
            keccak256(&self.dest.to_string().to_lowercase()),
            keccak256(&self.amount.to_string()),
            &signing_meta.nonce,
        );
        keccak256(items.abi_encode())
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NoOp;

impl Eip712 for NoOp {
    fn domain(&self, signing_meta: &SigningMeta) -> Eip712Domain {
        eip_712_domain()
    }

    fn struct_hash(&self, signing_meta: &SigningMeta) -> B256 {
        B256::ZERO
    }
}

pub fn hash(
    value: serde_json::Value,
    timestamp: u64,
    vault_address: Option<Address>,
    expires_after: Option<u64>,
) -> B256 {
    let mut bytes = rmp_serde::to_vec_named(&value).unwrap();
    bytes.extend(timestamp.to_be_bytes());
    if let Some(vault_address) = vault_address {
        bytes.push(1);
        bytes.extend(vault_address);
    } else {
        bytes.push(0);
    }
    if let Some(expires_after) = expires_after {
        bytes.push(0);
        bytes.extend(expires_after.to_be_bytes());
    }
    keccak256(bytes)
}
macro_rules! impl_user_signed_action {
    ($action:ident) => {
        impl Action for $action {
            fn signing_hash(&self, signing_meta: &SigningMeta) -> B256 {
                self.eip712_signing_hash(signing_meta)
            }
        }
    };
}

impl_user_signed_action!(UsdSend);

macro_rules! impl_l1_action {
    ($action:ident) => {
        impl Action for $action {
            fn signing_hash(&self, signing_meta: &SigningMeta) -> B256 {
                let signing_value = serde_json::to_value(self).unwrap();
                let hash = hash(signing_value, timestamp, vault_address, expires_after);
                let agent = Agent {
                    source: signing_meta.signing_chain.get_source(),
                    connectionId: hash,
                };
                agent.eip712_signing_hash(signing_meta)
            }
        }
    };
}

pub struct UsdSend {
    pub dest: Address,
    pub amount: f64,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(tag = "type")]
pub enum ActionKind {
    //UsdSend(UsdSend),
    #[serde(rename = "noop")]
    NoOp(NoOp),
}

#[test]
fn test_action_kind() {
    let action_kind = ActionKind::NoOp(NoOp);
    let serialized = serde_json::to_string(&action_kind).unwrap();
    println!("serialized = {}", serialized);
    let deserialized: ActionKind = serde_json::from_str(&serialized).unwrap();
    println!("deserialized = {:?}", deserialized);
}


#[test]
fn test_user_signed_action() {
    fn get_wallet() -> PrivateKeySigner {
        let priv_key = "e908f86dbb4d55ac876378565aafeabc187f6690f046459397b17d9b9a19688e";
        priv_key.parse::<PrivateKeySigner>().unwrap()
    }

    let send_action = UsdSend {
        dest: address!("0x0D1d9635D0640821d15e323ac8AdADfA9c111414"),
        amount: 100_000.69,
    };
    println!("send_action: {}", send_action.amount.to_string());

    let signing_meta = SigningMeta {
        signing_chain: SigningChain::Mainnet,
        nonce: 1690393044548,
        vault_address: None,
        expires_after: None,
    };

    let signature = send_action.sign(&get_wallet(), &signing_meta).unwrap();
    let expected_sig = "0x22cc3aa1849363a0d6aa1c216cca864ab392187c39b7891a45021c1c4f0211b0346e83c33d473578d7940551cc9911440c036466a12f6c899bb7fe307bc654261b";

    assert_eq!(signature.to_string(), expected_sig);
}
//### Improvement
//
//* Goals
//Improve ergonomics of passing a typed request by adding an ActionTr.
//
//Functionality to support:
//* Exchange Client requests can be typed without needing action kind wrapper.
//* Building signing data populates all common signing metadata. The inner dyn Action should just have parameters specific to the request.
//
//
//
//Create a Normalize Trait
//Create an Action Trait
//This Should be a super trait of Normalize and Into<ActionKind>
//
//Underlying action should implement Action Trait
//
//
//Action Trait should have a method to build the SigningData from the action.
//Normalize Impl should lowercase addresses.
//
//Add a Derive macro for L1Actions and UserSignedActions.
//derive macro should have a prop to allow additional pre-processing, like sorting.
//
//
//then the signing data can
//
//
//To Hash an action, call a function to normalize the struct. Generate this function with the derive macro.
//
//Action data needs to be formatted either during serialization or before it.
//
//
//Action Kind
//
//``` rust
//action
//
//```
