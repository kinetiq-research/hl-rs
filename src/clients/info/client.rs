use serde::Deserialize;

use crate::{
    error::ApiError,
    http::HttpClient,
    info::{client_builder::InfoClientBuilder, types::InfoRequest},
    prelude::{Error, Result},
    types::{Meta, PerpDex, SpotMeta},
    BaseUrl,
};

#[derive(Debug, Clone)]
pub struct InfoClient {
    pub(crate) http_client: HttpClient,
}

impl InfoClient {
    pub fn builder(base_url: BaseUrl) -> InfoClientBuilder {
        InfoClientBuilder::new(base_url)
    }

    async fn send_request<T: for<'a> Deserialize<'a>>(
        &self,
        info_request: InfoRequest,
    ) -> Result<T> {
        let data =
            serde_json::to_string(&info_request).map_err(|e| Error::JsonParse(e.to_string()))?;

        let return_data = self.http_client.post("/info", data).await?;
        serde_json::from_str(&return_data).map_err(|e| Error::JsonParse(e.to_string()))
    }

    pub async fn meta(&self) -> Result<Meta> {
        self.send_request(InfoRequest::Meta).await
    }

    pub async fn spot_meta(&self) -> Result<SpotMeta> {
        self.send_request(InfoRequest::SpotMeta).await
    }

    pub async fn perp_dexs(&self) -> Result<Vec<PerpDex>> {
        use serde_json::Value;

        let response: Vec<Value> = self.send_request(InfoRequest::PerpDexs).await?;

        if response.is_empty() {
            return Ok(vec![]);
        }

        // First element is error (null = no error)
        if !response[0].is_null() {
            let error = response[0]
                .as_str()
                .ok_or_else(|| Error::GenericParse("Expected string error".to_string()))?
                .to_string();
            return Err(Error::Api(ApiError::Other { message: error }));
        }

        // Rest are DEX objects
        let dexs: Vec<PerpDex> = response[1..]
            .iter()
            .map(|v| serde_json::from_value(v.clone()))
            .collect::<std::result::Result<Vec<_>, serde_json::Error>>()
            .map_err(|e| Error::JsonParse(e.to_string()))?;

        Ok(dexs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_perp_dexs() {
        let info_client = InfoClient::builder(BaseUrl::Testnet).build().unwrap();
        let perp_dexs = info_client.perp_dexs().await.unwrap();
        println!("{:?}", perp_dexs);
    }
}
