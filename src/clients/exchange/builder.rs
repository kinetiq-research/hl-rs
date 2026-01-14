use alloy::primitives::{Address, B256};

use crate::{
    eip712::Eip712,
    exchange::{Action, ActionKind, ExchangeClient},
    utils::{next_nonce, SigningData},
    Error, Result, SigningChain,
};

pub trait BuildAction {
    fn build(self, client: &ExchangeClient) -> Result<Action>;
}

impl BuildAction for ActionKind {
    /// Building the action creates signing data
    fn build(self, exchange_client: &ExchangeClient) -> Result<Action> {
        let timestamp = if self.is_l1_action() {
            next_nonce()
        } else {
            self.extract_timestamp().unwrap_or_else(|| next_nonce())
        };

        self.build_with_params(
            timestamp,
            exchange_client.vault_address,
            exchange_client.expires_after,
            exchange_client.base_url.get_signing_chain(),
        )
    }
}

impl ActionKind {
    pub fn is_l1_action(&self) -> bool {
        matches!(
            self,
            ActionKind::Order(_)
                | ActionKind::Cancel(_)
                | ActionKind::CancelByCloid(_)
                | ActionKind::BatchModify(_)
                | ActionKind::UpdateLeverage(_)
                | ActionKind::UpdateIsolatedMargin(_)
                | ActionKind::SpotUser(_)
                | ActionKind::VaultTransfer(_)
                | ActionKind::SetReferrer(_)
                | ActionKind::EvmUserModify(_)
                | ActionKind::ScheduleCancel(_)
                | ActionKind::ClaimRewards(_)
                | ActionKind::PerpDeploy(_)
        )
    }

    pub fn signing_data(
        &self,
        timestamp: u64,
        vault_address: Option<Address>,
        expires_after: Option<u64>,
        signing_chain: &SigningChain,
    ) -> Result<SigningData> {
        if self.is_l1_action() {
            // For perpDeploy, sign with vault_address=None even if it's set
            let hash_vault_address = match &self {
                ActionKind::PerpDeploy(_) => None,
                _ => vault_address,
            };

            Ok(SigningData::L1 {
                connection_id: self.hash(timestamp, hash_vault_address, expires_after)?,
                source: signing_chain.get_source(),
            })
        } else {
            let hash = self.extract_eip712_hash()?;
            Ok(SigningData::TypedData { hash })
        }
    }

    pub fn build_with_params(
        self,
        nonce: u64,
        vault_address: Option<Address>,
        expires_after: Option<u64>,
        signing_chain: &SigningChain,
    ) -> Result<Action> {
        let signing_data = self.signing_data(nonce, vault_address, expires_after, signing_chain)?;
        Ok(Action {
            action: self,
            nonce,
            vault_address,
            expires_after,
            signing_data,
        })
    }

    pub fn extract_eip712_hash(&self) -> Result<B256> {
        match self {
            ActionKind::UsdSend(usd_send) => Ok(usd_send.eip712_signing_hash()),
            ActionKind::Withdraw3(withdraw) => Ok(withdraw.eip712_signing_hash()),
            ActionKind::SpotSend(spot_send) => Ok(spot_send.eip712_signing_hash()),
            ActionKind::SendAsset(send_asset) => Ok(send_asset.eip712_signing_hash()),
            ActionKind::ApproveAgent(approve_agent) => Ok(approve_agent.eip712_signing_hash()),
            ActionKind::ApproveBuilderFee(approve_builder_fee) => {
                Ok(approve_builder_fee.eip712_signing_hash())
            }
            _ => Err(Error::GenericParse(
                "Action type not supported for typed data signing".to_string(),
            )),
        }
    }

    fn extract_timestamp(&self) -> Option<u64> {
        match self {
            ActionKind::UsdSend(usd_send) => Some(usd_send.time),
            ActionKind::Withdraw3(withdraw) => Some(withdraw.time),
            ActionKind::SpotSend(spot_send) => Some(spot_send.time),
            ActionKind::ApproveAgent(approve_agent) => Some(approve_agent.nonce),
            ActionKind::ApproveBuilderFee(approve_builder_fee) => Some(approve_builder_fee.nonce),
            ActionKind::SendAsset(send_asset) => Some(send_asset.nonce),
            _ => None,
        }
    }
}
