use alloy::primitives::{Address, B256, keccak256};

use crate::{
    Error, Result,
    eip712::Eip712,
    exchange::{Action, ActionKind, ExchangeClient, SigningData},
    utils::next_nonce,
};

pub trait BuildAction {
    fn build(self, client: &ExchangeClient) -> Result<Action>;
}

impl BuildAction for ActionKind {
    fn build(self, exchange_client: &ExchangeClient) -> Result<Action> {
        let vault_address = exchange_client.vault_address();
        let expires_after = exchange_client.expires_after();
        let is_l1_action = self.is_l1_action();

        let timestamp = if is_l1_action {
            next_nonce()
        } else {
            self.extract_timestamp().unwrap_or_else(|| next_nonce())
        };

        if is_l1_action {
            self.build_l1_action(
                exchange_client,
                timestamp,
                vault_address,
                expires_after.map(|e| e as i64),
            )
        } else {
            self.build_typed_data_action(
                exchange_client,
                timestamp,
                vault_address,
                expires_after.map(|e| e as i64),
            )
        }
    }
}

impl ActionKind {
    fn is_l1_action(&self) -> bool {
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

    fn build_l1_action(
        self,
        exchange_client: &ExchangeClient,
        timestamp: i64,
        vault_address: Option<Address>,
        expires_after: Option<i64>,
    ) -> Result<Action> {
        // For perpDeploy, sign with vault_address=None even if it's set
        let hash_vault_address = match &self {
            ActionKind::PerpDeploy(_) => None,
            _ => vault_address,
        };

        let connection_id = self.hash(timestamp, hash_vault_address, expires_after)?;
        let action_json =
            serde_json::to_value(&self).map_err(|e| Error::JsonParse(e.to_string()))?;

        Ok(Action {
            action: action_json,
            nonce: timestamp,
            vault_address,
            expires_after,
            signing_data: SigningData::L1 {
                connection_id,
                is_mainnet: exchange_client.is_mainnet(),
            },
            http_client: exchange_client.http_client().clone(),
        })
    }

    fn build_typed_data_action(
        self,
        exchange_client: &ExchangeClient,
        timestamp: i64,
        vault_address: Option<Address>,
        expires_after: Option<i64>,
    ) -> Result<Action> {
        let hash = self.extract_eip712_hash()?;
        let action_json =
            serde_json::to_value(&self).map_err(|e| Error::JsonParse(e.to_string()))?;

        Ok(Action {
            action: action_json,
            nonce: timestamp,
            vault_address,
            expires_after,
            signing_data: SigningData::TypedData { hash },
            http_client: exchange_client.http_client().clone(),
        })
    }

    fn extract_eip712_hash(&self) -> Result<B256> {
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

    fn extract_timestamp(&self) -> Option<i64> {
        match self {
            ActionKind::UsdSend(usd_send) => Some(usd_send.time as i64),
            ActionKind::Withdraw3(withdraw) => Some(withdraw.time as i64),
            ActionKind::SpotSend(spot_send) => Some(spot_send.time as i64),
            ActionKind::ApproveAgent(approve_agent) => Some(approve_agent.nonce as i64),
            ActionKind::ApproveBuilderFee(approve_builder_fee) => {
                Some(approve_builder_fee.nonce as i64)
            }
            ActionKind::SendAsset(send_asset) => Some(send_asset.nonce as i64),
            _ => None,
        }
    }
}
