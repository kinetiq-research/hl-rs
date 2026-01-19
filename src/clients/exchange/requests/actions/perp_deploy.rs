use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RegisterAsset {
    pub max_gas: Option<u64>,
    pub asset_request: RegisterAssetRequest,
    pub dex: String,
    pub schema: Option<PerpDexSchemaInput>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RegisterAssetRequest {
    pub coin: String,
    pub sz_decimals: u64,
    pub oracle_px: String,
    pub margin_table_id: u64,
    pub only_isolated: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PerpDexSchemaInput {
    pub full_name: String,
    pub collateral_token: u64,
    pub oracle_updater: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SetOracle {
    pub dex: String,
    pub oracle_pxs: Vec<(String, String)>,
    pub mark_pxs: Vec<Vec<(String, String)>>,
    pub external_perp_pxs: Vec<(String, String)>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SetFundingMultipliers {
    pub multipliers: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct SetFundingInterestRates {
    pub rates: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct SetMarginTableIds {
    pub ids: Vec<(String, i64)>,
}
/// Implements Serialize and Deserialize for a struct by flattening a Vec field as the whole struct.
/// Usage: flatten_vec!(StructName, field_name);
macro_rules! flatten_vec {
    ($struct_name:ident, $field:ident) => {
        impl ::serde::Serialize for $struct_name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: ::serde::Serializer,
            {
                let mut data = self.$field.clone();
                data.sort_by(|a, b| a.cmp(b));
                data.serialize(serializer)
            }
        }

        impl<'de> ::serde::Deserialize<'de> for $struct_name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: ::serde::Deserializer<'de>,
            {
                let vec_data = Vec::deserialize(deserializer)?;
                Ok($struct_name { $field: vec_data })
            }
        }
    };
}

flatten_vec!(SetMarginTableIds, ids);
flatten_vec!(SetOpenInterestCaps, caps);
flatten_vec!(SetFundingInterestRates, rates);
flatten_vec!(SetGrowthModes, modes);

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InsertMarginTable {
    pub dex: String,
    pub margin_table: RawMarginTable,
}

#[derive(Debug, Clone)]
pub struct SetGrowthModes {
    pub modes: Vec<(String, bool)>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RawMarginTable {
    pub description: String,
    pub margin_tiers: Vec<RawMarginTier>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RawMarginTier {
    pub lower_bound: i64,
    pub max_leverage: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HaltTrading {
    pub coin: String,
    pub is_halted: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SetFeeRecipient {
    pub dex: String,
    pub fee_recipient: String,
}

#[derive(Debug, Clone)]
pub struct SetOpenInterestCaps {
    pub caps: Vec<(String, u64)>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SetSubDeployers {
    pub dex: String,
    pub sub_deployers: Vec<SubDeployer>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SubDeployer {
    pub variant: Variant, // corresponds to a variant of PerpDeployAction. For example, "haltTrading" or "setOracle"
    pub user: String,
    pub allowed: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub enum Variant {
    RegisterAsset,
    SetOracle,
    SetFundingMultipliers,
    HaltTrading,
    SetMarginTableIds,
    SetFeeRecipient,
    SetOpenInterestCaps,
    InsertMarginTable,
    SetGrowthModes,
    SetFundingInterestRates
}

/// Wrapper that serializes with type: "perpDeploy"
/// and one of the action fields (registerAsset, setOracle, etc.)
#[derive(Debug, Clone)]
pub enum PerpDeploy {
    RegisterAsset(RegisterAsset),
    SetOracle(SetOracle),
    SetFundingMultipliers(SetFundingMultipliers),
    SetFundingInterestRates(SetFundingInterestRates),
    HaltTrading(HaltTrading),
    SetMarginTableIds(SetMarginTableIds),
    SetFeeRecipient(SetFeeRecipient),
    SetOpenInterestCaps(SetOpenInterestCaps),
    InsertMarginTable(InsertMarginTable),
    SetSubDeployers(SetSubDeployers),
    SetGrowthModes(SetGrowthModes),
}

impl Serialize for PerpDeploy {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("PerpDeploy", 1)?; // Changed from 2 to 1
        state.serialize_field("type", "perpDeploy")?;
        match self {
            PerpDeploy::RegisterAsset(v) => state.serialize_field("registerAsset", v)?,
            PerpDeploy::SetOracle(v) => state.serialize_field("setOracle", v)?,
            PerpDeploy::SetFundingMultipliers(v) => {
                state.serialize_field("setFundingMultipliers", v)?;
            }
            PerpDeploy::HaltTrading(v) => state.serialize_field("haltTrading", v)?,
            PerpDeploy::SetMarginTableIds(v) => state.serialize_field("setMarginTableIds", v)?,
            PerpDeploy::SetFeeRecipient(v) => state.serialize_field("setFeeRecipient", v)?,
            PerpDeploy::SetOpenInterestCaps(v) => {
                state.serialize_field("setOpenInterestCaps", v)?;
            }
            PerpDeploy::InsertMarginTable(v) => state.serialize_field("insertMarginTable", v)?,
            PerpDeploy::SetSubDeployers(v) => state.serialize_field("setSubDeployers", v)?,
            PerpDeploy::SetGrowthModes(v) => state.serialize_field("setGrowthModes", v)?,
            PerpDeploy::SetFundingInterestRates(v) => state.serialize_field("setFundingInterestRates", v)?,
        }
        state.end()
    }
}

impl<'de> Deserialize<'de> for PerpDeploy {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, Visitor};
        use std::fmt;

        struct PerpDeployVisitor;

        impl<'de> Visitor<'de> for PerpDeployVisitor {
            type Value = PerpDeploy;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a perpDeploy action")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut type_val: Option<String> = None;
                let mut register_asset: Option<RegisterAsset> = None;
                let mut set_oracle: Option<SetOracle> = None;
                let mut set_funding_multipliers: Option<SetFundingMultipliers> = None;
                let mut halt_trading: Option<HaltTrading> = None;
                let mut set_margin_table_ids: Option<SetMarginTableIds> = None;
                let mut set_fee_recipient: Option<SetFeeRecipient> = None;
                let mut set_open_interest_caps: Option<SetOpenInterestCaps> = None;
                let mut insert_margin_table: Option<InsertMarginTable> = None;
                let mut set_sub_deployers: Option<SetSubDeployers> = None;
                let mut set_growth_mode: Option<SetGrowthModes> = None;
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "type" => {
                            type_val = Some(map.next_value()?);
                        }
                        "registerAsset" => {
                            register_asset = Some(map.next_value()?);
                        }
                        "setOracle" => {
                            set_oracle = Some(map.next_value()?);
                        }
                        "setFundingMultipliers" => {
                            set_funding_multipliers = Some(map.next_value()?);
                        }
                        "haltTrading" => {
                            halt_trading = Some(map.next_value()?);
                        }
                        "setMarginTableIds" => {
                            set_margin_table_ids = Some(map.next_value()?);
                        }
                        "setFeeRecipient" => {
                            set_fee_recipient = Some(map.next_value()?);
                        }
                        "setOpenInterestCaps" => {
                            set_open_interest_caps = Some(map.next_value()?);
                        }
                        "insertMarginTable" => {
                            insert_margin_table = Some(map.next_value()?);
                        }
                        "setSubDeployers" => {
                            set_sub_deployers = Some(map.next_value()?);
                        }
                        "setGrowthModes" => {
                            set_growth_mode = Some(map.next_value()?);
                        }
                        _ => {
                            let _ = map.next_value::<serde::de::IgnoredAny>()?;
                        }
                    }
                }

                if let Some(t) = type_val.as_deref() {
                    if t != "perpDeploy" {
                        return Err(de::Error::invalid_value(
                            serde::de::Unexpected::Str(t),
                            &"perpDeploy",
                        ));
                    }
                }

                if let Some(v) = register_asset {
                    Ok(PerpDeploy::RegisterAsset(v))
                } else if let Some(v) = set_oracle {
                    Ok(PerpDeploy::SetOracle(v))
                } else if let Some(v) = set_funding_multipliers {
                    Ok(PerpDeploy::SetFundingMultipliers(v))
                } else if let Some(v) = halt_trading {
                    Ok(PerpDeploy::HaltTrading(v))
                } else if let Some(v) = set_margin_table_ids {
                    Ok(PerpDeploy::SetMarginTableIds(v))
                } else if let Some(v) = set_fee_recipient {
                    Ok(PerpDeploy::SetFeeRecipient(v))
                } else if let Some(v) = set_open_interest_caps {
                    Ok(PerpDeploy::SetOpenInterestCaps(v))
                } else if let Some(v) = insert_margin_table {
                    Ok(PerpDeploy::InsertMarginTable(v))
                } else if let Some(v) = set_sub_deployers {
                    Ok(PerpDeploy::SetSubDeployers(v))
                } else if let Some(v) = set_growth_mode {
                    Ok(PerpDeploy::SetGrowthModes(v))
                } else {
                    Err(de::Error::missing_field(
                        "one of the perpDeploy action fields",
                    ))
                }
            }
        }

        deserializer.deserialize_map(PerpDeployVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perp_deploy_serialize() {
        let perp_deploy = PerpDeploy::RegisterAsset(RegisterAsset {
            max_gas: None,
            asset_request: RegisterAssetRequest {
                coin: "BTC".to_string(),
                sz_decimals: 8,
                oracle_px: "100".to_string(),
                margin_table_id: 1,
                only_isolated: true,
            },
            dex: "a".to_string(),
            schema: None,
        });

        let serialized = serde_json::to_string(&perp_deploy).unwrap();
        assert_eq!(
            serialized,
            "{\"type\":\"perpDeploy\",\"registerAsset\":{\"maxGas\":null,\"assetRequest\":{\"coin\":\"BTC\",\"szDecimals\":8,\"oraclePx\":\"100\",\"marginTableId\":1,\"onlyIsolated\":true},\"dex\":\"a\",\"schema\":null}}"
        );
    }
}
