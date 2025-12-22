use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};

use crate::{prelude::Result, BaseUrl, Error};

#[derive(Deserialize, Debug)]
struct ErrorData {
    data: String,
    code: u16,
    msg: String,
}

#[derive(Debug, Clone)]
pub struct HttpClient {
    pub client: Client,
    pub base_url: String,
}

async fn parse_response(response: Response) -> Result<String> {
    let status_code = response.status().as_u16();
    let text = response
        .text()
        .await
        .map_err(|e| Error::GenericRequest(e.to_string()))?;

    if status_code < 400 {
        return Ok(text);
    }

    tracing::debug!(target: "hl_rs::http_client", status_code=status_code, body=text, "Error response received");

    let error_data = serde_json::from_str::<ErrorData>(&text);
    if (400..500).contains(&status_code) {
        let client_error = match error_data {
            Ok(error_data) => Error::ClientRequest {
                status_code,
                error_code: Some(error_data.code),
                error_message: error_data.msg,
                error_data: Some(error_data.data),
            },
            Err(err) => Error::ClientRequest {
                status_code,
                error_message: text,
                error_code: None,
                error_data: Some(err.to_string()),
            },
        };
        return Err(client_error);
    }

    Err(Error::ServerRequest {
        status_code,
        error_message: text,
    })
}

impl HttpClient {
    #[tracing::instrument(skip(self, data))]
    pub async fn post<T: Serialize>(&self, url_path: &'static str, data: T) -> Result<String> {
        let full_url = format!("{}{url_path}", self.base_url);

        // Serialize the payload for logging
        let payload_json =
            serde_json::to_string(&data).map_err(|e| Error::SerializationFailure(e.to_string()))?;
        tracing::debug!(target: "hl_rs::http_client", url=full_url, payload=payload_json, "Sending POST request");

        let res = self
            .client
            .post(&full_url)
            .json(&data)
            .send()
            .await
            .map_err(|e| Error::GenericRequest(e.to_string()))?;
        tracing::debug!(target: "hl_rs::http_client", res=?res, "Response");
        parse_response(res).await
    }

    pub fn is_mainnet(&self) -> bool {
        self.base_url == BaseUrl::Mainnet.get_url()
    }
}
