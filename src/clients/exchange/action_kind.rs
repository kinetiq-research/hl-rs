use alloy::primitives::{keccak256, Address, B256};
use serde::{Deserialize, Serialize, Serializer};

use crate::{
    exchange::requests::{
        ApproveAgent, ApproveBuilderFee, BulkCancel, BulkCancelCloid, BulkModify, BulkOrder,
        ClaimRewards, EvmUserModify, PerpDeploy, ScheduleCancel, SendAsset, SetReferrer, SpotSend,
        SpotUser, UpdateIsolatedMargin, UpdateLeverage, UsdSend, VaultTransfer, Withdraw3,
    },
    Error, Result,
};

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "camelCase")]
pub enum ActionKind {
    UsdSend(UsdSend),
    UpdateLeverage(UpdateLeverage),
    UpdateIsolatedMargin(UpdateIsolatedMargin),
    Order(BulkOrder),
    Cancel(BulkCancel),
    CancelByCloid(BulkCancelCloid),
    BatchModify(BulkModify),
    ApproveAgent(ApproveAgent),
    Withdraw3(Withdraw3),
    SpotUser(SpotUser),
    SendAsset(SendAsset),
    VaultTransfer(VaultTransfer),
    SpotSend(SpotSend),
    SetReferrer(SetReferrer),
    ApproveBuilderFee(ApproveBuilderFee),
    EvmUserModify(EvmUserModify),
    ScheduleCancel(ScheduleCancel),
    ClaimRewards(ClaimRewards),
    PerpDeploy(PerpDeploy),
}

impl Serialize for ActionKind {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;

        match self {
            ActionKind::PerpDeploy(perp_deploy) => {
                let mut state = serializer.serialize_struct("ActionKind", 2)?;
                state.serialize_field("type", "perpDeploy")?;
                match perp_deploy {
                    PerpDeploy::RegisterAsset(v) => state.serialize_field("registerAsset", v)?,
                    PerpDeploy::SetOracle(v) => state.serialize_field("setOracle", v)?,
                    PerpDeploy::SetFundingMultipliers(v) => {
                        state.serialize_field("setFundingMultipliers", v)?;
                    }
                    PerpDeploy::HaltTrading(v) => state.serialize_field("haltTrading", v)?,
                    PerpDeploy::SetMarginTableIds(v) => {
                        state.serialize_field("setMarginTableIds", v)?
                    }
                    PerpDeploy::SetFeeRecipient(v) => {
                        state.serialize_field("setFeeRecipient", v)?
                    }
                    PerpDeploy::SetOpenInterestCaps(v) => {
                        state.serialize_field("setOpenInterestCaps", v)?;
                    }
                    PerpDeploy::InsertMarginTable(v) => {
                        state.serialize_field("insertMarginTable", v)?
                    }
                    PerpDeploy::SetSubDeployers(v) => {
                        state.serialize_field("setSubDeployers", v)?
                    }
                    PerpDeploy::SetFundingInterestRates(v) => {
                        state.serialize_field("setFundingInterestRates", v)?
                    }
                    PerpDeploy::SetGrowthModes(v) => state.serialize_field("setGrowthModes", v)?,
                }
                state.end()
            }
            _ => {
                let mut map = serde_json::Map::new();
                let (type_name, value) = match self {
                    ActionKind::UsdSend(v) => ("usdSend", serde_json::to_value(v).unwrap()),
                    ActionKind::UpdateLeverage(v) => {
                        ("updateLeverage", serde_json::to_value(v).unwrap())
                    }
                    ActionKind::UpdateIsolatedMargin(v) => {
                        ("updateIsolatedMargin", serde_json::to_value(v).unwrap())
                    }
                    ActionKind::Order(v) => ("order", serde_json::to_value(v).unwrap()),
                    ActionKind::Cancel(v) => ("cancel", serde_json::to_value(v).unwrap()),
                    ActionKind::CancelByCloid(v) => {
                        ("cancelByCloid", serde_json::to_value(v).unwrap())
                    }
                    ActionKind::BatchModify(v) => ("batchModify", serde_json::to_value(v).unwrap()),
                    ActionKind::ApproveAgent(v) => {
                        ("approveAgent", serde_json::to_value(v).unwrap())
                    }
                    ActionKind::Withdraw3(v) => ("withdraw3", serde_json::to_value(v).unwrap()),
                    ActionKind::SpotUser(v) => ("spotUser", serde_json::to_value(v).unwrap()),
                    ActionKind::SendAsset(v) => ("sendAsset", serde_json::to_value(v).unwrap()),
                    ActionKind::VaultTransfer(v) => {
                        ("vaultTransfer", serde_json::to_value(v).unwrap())
                    }
                    ActionKind::SpotSend(v) => ("spotSend", serde_json::to_value(v).unwrap()),
                    ActionKind::SetReferrer(v) => ("setReferrer", serde_json::to_value(v).unwrap()),
                    ActionKind::ApproveBuilderFee(v) => {
                        ("approveBuilderFee", serde_json::to_value(v).unwrap())
                    }
                    ActionKind::EvmUserModify(v) => {
                        ("evmUserModify", serde_json::to_value(v).unwrap())
                    }
                    ActionKind::ScheduleCancel(v) => {
                        ("scheduleCancel", serde_json::to_value(v).unwrap())
                    }
                    ActionKind::ClaimRewards(v) => {
                        ("claimRewards", serde_json::to_value(v).unwrap())
                    }
                    ActionKind::PerpDeploy(_) => unreachable!(),
                };
                map.insert(
                    "type".to_string(),
                    serde_json::Value::String(type_name.to_string()),
                );
                map.insert(type_name.to_string(), value);
                map.serialize(serializer)
            }
        }
    }
}

impl ActionKind {
    pub fn hash(
        &self,
        timestamp: u64,
        vault_address: Option<Address>,
        expires_after: Option<u64>,
    ) -> Result<B256> {
        let mut bytes =
            rmp_serde::to_vec_named(self).map_err(|e| Error::RmpParse(e.to_string()))?;
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
        Ok(keccak256(bytes))
    }
}
