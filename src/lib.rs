pub mod types;

mod abi_value;
pub mod actions;
#[cfg(feature = "client")]
pub mod clients;
mod consts;
mod error;
#[cfg(feature = "client")]
mod http;
mod prelude;

#[cfg(feature = "client")]
pub use clients::{
    exchange::responses::ExchangeResponse,
    info::{self, InfoClient},
    ExchangeClient,
};

pub use abi_value::{AbiResult, ToAbiValue};
pub use alloy::dyn_abi::DynSolType;
pub use consts::{MAINNET_API_URL, MAINNET_WS_URL, TESTNET_API_URL, TESTNET_WS_URL};
pub use error::Error;
pub use prelude::Result;
pub use types::{BaseUrl, SigningChain};

#[cfg(feature = "ws")]
pub use clients::ws::{responses, Price, Subscription, WsActiveAssetCtx, WsClient, WsMessage};

pub use actions::*;
