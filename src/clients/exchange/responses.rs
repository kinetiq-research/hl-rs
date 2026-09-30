use serde::{Deserialize, Serialize};

use crate::{error::ApiError, Error};

/// Raw API response wrapper: status ok/err + response body.
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
    Resting(RestingOrder),
    Filled(FilledOrder),
    /// API returned an error for this order (e.g. insufficient balance)
    Error(String),
    /// Catch-all for unknown response variants
    #[serde(other)]
    Unknown,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ExchangeDataStatuses {
    pub statuses: Vec<ExchangeDataStatus>,
}

/// Exchange API response. `data` is generic JSON so different action types
/// (order, setGlobal, etc.) can return different structures without deserialization failures.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ExchangeResponse {
    #[serde(rename = "type")]
    pub response_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl ExchangeResponse {
    /// Parsed order statuses when `response_type` is `"order"` and `data` has `statuses`.
    /// Returns None for non-order responses. Corrupt order payloads are also
    /// reported as None by this legacy helper; use `try_order_data` when the
    /// distinction matters.
    pub fn order_data(&self) -> Option<ExchangeDataStatuses> {
        self.try_order_data().unwrap_or(None)
    }

    /// Strict variant that distinguishes "not an order response" (Ok(None))
    /// from "order response with corrupt payload" (Err).
    pub fn try_order_data(&self) -> Result<Option<ExchangeDataStatuses>, Error> {
        if self.response_type != "order" {
            return Ok(None);
        }
        let Some(value) = self.data.as_ref() else {
            return Ok(None);
        };
        serde_json::from_value(value.clone())
            .map(Some)
            .map_err(|e| Error::JsonParse(e.to_string()))
    }

    /// Messages array when `response_type` is `"setGlobal"` and `data` is `["msg1", "msg2", ...]`.
    pub fn set_global_messages(&self) -> Option<Vec<String>> {
        if self.response_type != "setGlobal" {
            return None;
        }
        self.data.as_ref().and_then(|v| {
            let arr = v.as_array()?;
            let mut msgs = Vec::with_capacity(arr.len());
            for item in arr {
                msgs.push(item.as_str()?.to_string());
            }
            Some(msgs)
        })
    }
}
