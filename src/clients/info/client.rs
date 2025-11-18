use serde::Deserialize;

use crate::{
    http::HttpClient,
    info::{client_builder::InfoClientBuilder, types::InfoRequest},
    prelude::{Error, Result},
    types::{Meta, SpotMeta},
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
}
