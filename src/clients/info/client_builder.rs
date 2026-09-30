use reqwest::Client;

use crate::{http::HttpClient, info::InfoClient, prelude::Result, types::BaseUrl};

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
        let http_client = self.http_client.unwrap_or_else(|| {
            // Bounded timeout so info requests cannot hang indefinitely.
            let client = Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_default();
            HttpClient {
                client,
                base_url: self.base_url.get_url(),
            }
        });

        Ok(InfoClient { http_client })
    }
}
