use alloy::{
    dyn_abi::Eip712Domain,
    primitives::{Address, B256},
    signers::{local::PrivateKeySigner, Signature, SignerSync},
    sol,
    sol_types::{eip712_domain, SolStruct},
};

use serde::{Deserialize, Serialize};

use crate::{eip712::Eip712, exchange::SignedAction, Error, Result, SigningChain};

/// Enum representing data needed for signing an action.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum SigningData {
    /// L1 actions require a connection_id and network type.
    L1 { connection_id: B256, source: String },
    /// Typed data actions require only the EIP-712 hash.
    TypedData { hash: B256 },
}

impl SigningData {
    pub fn signing_hash(&self) -> B256 {
        match self {
            SigningData::L1 {
                connection_id,
                source,
            } => <Agent as Eip712>::eip712_signing_hash(&Agent {
                connectionId: *connection_id,
                source: source.to_owned(),
            }),
            SigningData::TypedData { hash } => *hash,
        }
    }
    pub fn sign(&self, wallet: &PrivateKeySigner) -> Result<Signature> {
        let hash = self.signing_hash();
        wallet
            .sign_hash_sync(&hash)
            .map_err(|e| Error::SignatureFailure(e.to_string()))
    }
}

sol! {
    #[derive(Debug)]
    struct Agent {
        string source;
        bytes32 connectionId;
    }

}

impl Eip712 for Agent {
    fn domain(&self) -> Eip712Domain {
        eip712_domain! {
            name: "Exchange",
            version: "1",
            chain_id: 1337,
            verifying_contract: Address::ZERO,
        }
    }
    fn struct_hash(&self) -> B256 {
        self.eip712_hash_struct()
    }
}

pub fn recover_action(
    signing_chain: &SigningChain,
    signed_action: &SignedAction,
) -> Result<Address> {
    let SignedAction {
        action,
        nonce,
        vault_address,
        expires_after,
        signature,
    } = signed_action;

    let action =
        action
            .clone()
            .build_with_params(*nonce, *vault_address, *expires_after, signing_chain)?;

    signature
        .recover_address_from_prehash(&action.signing_data.signing_hash())
        .map_err(|e| Error::RecoverAddressFailure(e.to_string()))
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use crate::{
        exchange::{
            builder::BuildAction,
            requests::{UsdSend, Withdraw3},
            ActionKind, ExchangeClient,
        },
        BaseUrl,
    };

    fn get_wallet() -> Result<PrivateKeySigner> {
        let priv_key = "e908f86dbb4d55ac876378565aafeabc187f6690f046459397b17d9b9a19688e";
        priv_key
            .parse::<PrivateKeySigner>()
            .map_err(|e| Error::Wallet(e.to_string()))
    }

    #[test]
    fn test_sign_l1_action() -> Result<()> {
        let wallet = get_wallet()?;
        let connection_id =
            B256::from_str("0xde6c4037798a4434ca03cd05f00e3b803126221375cd1e7eaaaf041768be06eb")
                .map_err(|e| Error::GenericParse(e.to_string()))?;

        let expected_mainnet_sig = "0xfa8a41f6a3fa728206df80801a83bcbfbab08649cd34d9c0bfba7c7b2f99340f53a00226604567b98a1492803190d65a201d6805e5831b7044f17fd530aec7841c";
        let signing_data = SigningData::L1 {
            connection_id,
            source: SigningChain::Mainnet.get_source(),
        };
        assert_eq!(
            signing_data.sign(&wallet)?.to_string(),
            expected_mainnet_sig
        );

        let expected_testnet_sig = "0x1713c0fc661b792a50e8ffdd59b637b1ed172d9a3aa4d801d9d88646710fb74b33959f4d075a7ccbec9f2374a6da21ffa4448d58d0413a0d335775f680a881431c";
        let signing_data = SigningData::L1 {
            connection_id,
            source: SigningChain::Testnet.get_source(),
        };

        assert_eq!(
            signing_data.sign(&wallet)?.to_string(),
            expected_testnet_sig
        );
        Ok(())
    }

    #[test]
    fn test_sign_usd_transfer_action() -> Result<()> {
        let wallet = get_wallet()?;

        let usd_send = UsdSend {
            signature_chain_id: 421614,
            hyperliquid_chain: BaseUrl::Testnet.get_hyperliquid_chain(),
            destination: "0x0D1d9635D0640821d15e323ac8AdADfA9c111414".to_string(),
            amount: "1".to_string(),
            time: 1690393044548,
        };

        let signing_data = SigningData::TypedData {
            hash: usd_send.eip712_signing_hash(),
        };

        let expected_sig = "0x214d507bbdaebba52fa60928f904a8b2df73673e3baba6133d66fe846c7ef70451e82453a6d8db124e7ed6e60fa00d4b7c46e4d96cb2bd61fd81b6e8953cc9d21b";

        assert_eq!(signing_data.sign(&wallet)?.to_string(), expected_sig);
        Ok(())
    }

    #[test]
    fn test_sign_withdraw_from_bridge_action() -> Result<()> {
        let wallet = get_wallet()?;

        let usd_send = Withdraw3 {
            signature_chain_id: 421614,
            hyperliquid_chain: BaseUrl::Testnet.get_hyperliquid_chain(),
            destination: "0x0D1d9635D0640821d15e323ac8AdADfA9c111414".to_string(),
            amount: "1".to_string(),
            time: 1690393044548,
        };
        let signing_data = SigningData::TypedData {
            hash: usd_send.eip712_signing_hash(),
        };

        let expected_sig = "0xb3172e33d2262dac2b4cb135ce3c167fda55dafa6c62213564ab728b9f9ba76b769a938e9f6d603dae7154c83bf5a4c3ebab81779dc2db25463a3ed663c82ae41c";
        assert_eq!(signing_data.sign(&wallet)?.to_string(), expected_sig);
        Ok(())
    }

    #[tokio::test]
    async fn test_recover_action() -> Result<()> {
        let wallet = get_wallet()?;
        let expected_address = wallet.address();
        let exchange_client = ExchangeClient::builder(BaseUrl::Testnet)
            .build()
            .await
            .unwrap();

        let usd_send = UsdSend {
            signature_chain_id: 421614,
            hyperliquid_chain: "Testnet".to_string(),
            destination: "0x0D1d9635D0640821d15e323ac8AdADfA9c111414".to_string(),
            amount: "1".to_string(),
            time: 1690393044548,
        };
        let action = ActionKind::UsdSend(usd_send.clone())
            .build(&exchange_client)
            .unwrap();

        let signed_action = action.sign(&wallet).unwrap();
        let recovered_address = signed_action.recover_user(&exchange_client)?;

        assert_eq!(recovered_address, expected_address);
        Ok(())
    }

    #[tokio::test]
    async fn test_sign_register_asset_l1_action() -> Result<()> {
        use crate::exchange::{
            requests::{PerpDeploy, PerpDexSchemaInput, RegisterAsset, RegisterAssetRequest},
            ActionKind,
        };

        // Use the same wallet key as Python SDK test
        let priv_key = "0x0123456789012345678901234567890123456789012345678901234567890123";
        let wallet = priv_key
            .parse::<PrivateKeySigner>()
            .map_err(|e| Error::Wallet(e.to_string()))?;

        // Create RegisterAsset action with same parameters as Python SDK test (without schema)
        let register_asset_no_schema = RegisterAsset {
            max_gas: Some(1000000000000),
            asset_request: RegisterAssetRequest {
                coin: "ddd:TEST0".to_string(),
                sz_decimals: 2,
                oracle_px: "10.0".to_string(),
                margin_table_id: 10,
                only_isolated: true,
            },
            dex: "ddd".to_string(),
            schema: None,
        };

        // Build action with nonce=0 (same as Python SDK test)
        let action_kind =
            ActionKind::PerpDeploy(PerpDeploy::RegisterAsset(register_asset_no_schema));
        let action = action_kind.build_with_params(0, None, None, &SigningChain::Testnet)?;

        // Sign the action
        let signed_action = action.sign(&wallet)?;

        // Expected signature from Python SDK test (testnet, without schema)
        // R: 0x90ce842264d3024c2fcd76cec1283c9afc76e0b67d27018d90dd2d52f37ddb83
        // S: 0x66c30d2676f5c057eda65bc7e8633ace0b3a24d9a4f6a03fed462035b0e018e7
        // V: 28
        // Full signature string: 0x90ce842264d3024c2fcd76cec1283c9afc76e0b67d27018d90dd2d52f37ddb8366c30d2676f5c057eda65bc7e8633ace0b3a24d9a4f6a03fed462035b0e018e71c
        let expected_sig_no_schema = "0x90ce842264d3024c2fcd76cec1283c9afc76e0b67d27018d90dd2d52f37ddb8366c30d2676f5c057eda65bc7e8633ace0b3a24d9a4f6a03fed462035b0e018e71c";

        // Compare the full signature string
        assert_eq!(
            signed_action.signature.to_string(),
            expected_sig_no_schema,
            "Signature mismatch for RegisterAsset l1_action without schema"
        );

        // Test with schema
        let wallet_address_lower = wallet.address().to_string().to_lowercase();
        let register_asset_with_schema = RegisterAsset {
            max_gas: Some(1000000000000),
            asset_request: RegisterAssetRequest {
                coin: "ddd:TEST0".to_string(),
                sz_decimals: 2,
                oracle_px: "10.0".to_string(),
                margin_table_id: 10,
                only_isolated: true,
            },
            dex: "ddd".to_string(),
            schema: Some(PerpDexSchemaInput {
                full_name: "Test DEX".to_string(),
                collateral_token: 1452,
                oracle_updater: Some(wallet_address_lower),
            }),
        };

        let action_kind_with_schema =
            ActionKind::PerpDeploy(PerpDeploy::RegisterAsset(register_asset_with_schema));
        let action_with_schema =
            action_kind_with_schema.build_with_params(0, None, None, &SigningChain::Testnet)?;
        let signed_action_with_schema = action_with_schema.sign(&wallet)?;

        // Expected signature from Python SDK test (testnet, with schema)
        // R: 0xa52d17bc32add97d991798ac20d224501c8b01b82e07e336bd98049b905702cc
        // S: 0x329653c8eaed0c2e28241112a9a9fe0965a27540a25c725992d452c2b5fc17c3
        // V: 27
        // Full signature string: 0xa52d17bc32add97d991798ac20d224501c8b01b82e07e336bd98049b905702cc329653c8eaed0c2e28241112a9a9fe0965a27540a25c725992d452c2b5fc17c31b
        let expected_sig_with_schema = "0xa52d17bc32add97d991798ac20d224501c8b01b82e07e336bd98049b905702cc329653c8eaed0c2e28241112a9a9fe0965a27540a25c725992d452c2b5fc17c31b";

        assert_eq!(
            signed_action_with_schema.signature.to_string(),
            expected_sig_with_schema,
            "Signature mismatch for RegisterAsset l1_action with schema"
        );

        Ok(())
    }
}
