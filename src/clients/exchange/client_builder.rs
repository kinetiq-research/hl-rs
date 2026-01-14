use std::collections::HashMap;

use alloy::{primitives::Address, signers::local::PrivateKeySigner};
use reqwest::Client;

use crate::{
    http::HttpClient,
    info::InfoClient,
    prelude::Result,
    types::{BaseUrl, CoinToAsset, Meta},
    ExchangeClient,
};

#[derive(Debug, Clone)]
pub struct ExchangeClientBuilder {
    base_url: BaseUrl,
    signer_private_key: Option<PrivateKeySigner>,
    meta: Option<Meta>,
    vault_address: Option<Address>,
    expires_after: Option<u64>,
    info_client: Option<InfoClient>,
}

impl ExchangeClientBuilder {
    pub fn new(base_url: BaseUrl) -> Self {
        Self {
            base_url,
            signer_private_key: None,
            meta: None,
            vault_address: None,
            expires_after: None,
            info_client: None,
        }
    }

    pub fn signer_private_key(mut self, signer: PrivateKeySigner) -> Self {
        self.signer_private_key = Some(signer);
        self
    }

    pub fn meta(mut self, meta: Meta) -> Self {
        self.meta = Some(meta);
        self
    }

    pub fn vault_address(mut self, address: Address) -> Self {
        self.vault_address = Some(address);
        self
    }

    pub fn expires_after(mut self, expires: u64) -> Self {
        self.expires_after = Some(expires);
        self
    }

    pub fn info_client(mut self, info_client: InfoClient) -> Self {
        self.info_client = Some(info_client);
        self
    }

    pub async fn build(mut self) -> Result<ExchangeClient> {
        let http_client = HttpClient {
            client: Client::default(),
            base_url: self.base_url.get_url(),
        };
        let info_client = if let Some(client) = self.info_client.take() {
            client
        } else {
            InfoClient::builder(self.base_url.clone()).build()?
        };

        let meta = if let Some(meta) = self.meta.take() {
            meta
        } else {
            info_client.meta().await?
        };

        let coin_to_asset =
            Self::derive_coin_to_asset_from_meta(meta.clone(), &info_client).await?;

        Ok(ExchangeClient {
            http_client,
            base_url: self.base_url,
            vault_address: self.vault_address,
            meta: Some(meta),
            expires_after: self.expires_after,
            coin_to_asset,
        })
    }

    async fn derive_coin_to_asset_from_meta(
        meta: Meta,
        info_client: &InfoClient,
    ) -> Result<CoinToAsset> {
        let mut coin_to_asset = HashMap::new();
        for (asset_ind, asset) in meta.universe.iter().enumerate() {
            coin_to_asset.insert(asset.name.clone(), asset_ind as u32);
        }

        coin_to_asset = info_client
            .spot_meta()
            .await?
            .add_pair_and_name_to_index_map(coin_to_asset);

        Ok(CoinToAsset::new(coin_to_asset))
    }
}
