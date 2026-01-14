use serde::{Deserialize, Serialize};

use crate::{error::ApiError, Error};

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(tag = "status", content = "response")]
pub enum ExchangeResponseStatusRaw {
    #[serde(rename = "ok")]
    Ok(ExchangeResponse),
    #[serde(rename = "err")]
    Err(String),
}

impl ExchangeResponseStatusRaw {
    pub fn into_result(self) -> Result<ExchangeResponse, Error> {
        match self {
            ExchangeResponseStatusRaw::Ok(response) => Ok(response),
            ExchangeResponseStatusRaw::Err(msg) => {
                let api_error = if msg.contains("insufficient staked HYPE")
                    || msg.contains("insufficient staked")
                {
                    ApiError::InsufficientStakedHype { message: msg }
                } else {
                    ApiError::Other { message: msg }
                };
                Err(Error::Api(api_error))
                // } else if msg.contains("User or API Wallet") || msg.contains("does not exist") {
                //     let address =
                //         extract_address_from_error(&msg).unwrap_or_else(|| "unknown".to_string());
                //     ApiError::WalletNotFound { address }
                // } else if msg.contains("signature") || msg.contains("Signature") {
                //     ApiError::SignatureMismatch { message: msg }
                // } else {
                //     ApiError::Other { message: msg }
                // };
                // Err(Error::ApiError(api_error))
            }
        }
    }
}

//fn extract_address_from_error(msg: &str) -> Option<String> {
//    msg.split_whitespace()
//        .find(|s| s.starts_with("0x") && s.len() == 42)
//        .map(|s| s.to_string())
//}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct RestingOrder {
    pub oid: u64,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FilledOrder {
    pub total_sz: String,
    pub avg_px: String,
    pub oid: u64,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub enum ExchangeDataStatus {
    Success,
    WaitingForFill,
    WaitingForTrigger,
    //    Error(String),
    Resting(RestingOrder),
    Filled(FilledOrder),
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ExchangeDataStatuses {
    pub statuses: Vec<ExchangeDataStatus>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ExchangeResponse {
    #[serde(rename = "type")]
    pub response_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<ExchangeDataStatuses>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "status", content = "response")]
pub enum ExchangeResponseStatus {
    Ok(ExchangeResponse),
    Err(String),
}
