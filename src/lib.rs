// A panic in this crate takes its caller down with it, and a signer built with
// `panic = "abort"` halts. Library code returns `Error` instead; a site that truly
// cannot fail carries `#[allow(clippy::..., reason = "...")]` stating why, and
// `allow_attributes_without_reason` rejects an allow without one. Arithmetic that can
// overflow or divide by zero (integers, and `rust_decimal::Decimal`, whose operators
// panic) goes through `checked_*`. Tests may panic. CI runs clippy with these lints
// for the actions-only and the client build, plus
// `.github/scripts/no_panic_outside_clippy.py` for what clippy cannot see: macro
// bodies, and calls that panic on a bad length or condition (`assert!`, `from_slice`,
// `copy_from_slice`, `split_at`), each of which needs a `// cannot panic: ...`
// comment on its line or the line above.
#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable,
        clippy::todo,
        clippy::unimplemented,
        clippy::indexing_slicing,
        clippy::string_slice,
        clippy::arithmetic_side_effects,
        clippy::allow_attributes_without_reason
    )
)]

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
