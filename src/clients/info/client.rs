use reqwest::Client;
use serde::Deserialize;

use crate::{
    http::HttpClient,
    info::types::InfoRequest,
    prelude::{Error, Result},
    types::{Meta, SpotMeta},
    BaseUrl,
};

#[derive(Debug, Clone)]
pub struct InfoClientBuilder {
    base_url: BaseUrl,
    http_client: Option<HttpClient>,
}

impl InfoClientBuilder {
    pub fn new(base_url: BaseUrl) -> Self {
        Self {
            base_url,
            http_client: None,
        }
    }

    pub fn http_client(mut self, http_client: HttpClient) -> Self {
        self.http_client = Some(http_client);
        self
    }

    pub fn build(self) -> Result<InfoClient> {
        let http_client = self.http_client.unwrap_or(HttpClient {
            client: Client::default(),
            base_url: self.base_url.get_url(),
        });

        Ok(InfoClient { http_client })
    }
}

#[derive(Debug, Clone)]
pub struct InfoClient {
    http_client: HttpClient,
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
