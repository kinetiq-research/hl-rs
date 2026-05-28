use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum Error {
    #[error(transparent)]
    Api(#[from] ApiError),

    #[error("Signer not set. Call `ExchangeClientV2::with_signer(...)` before sending actions.")]
    SignerNotSet,

    #[error(
        "Client error: status code: {status_code}, error code: {error_code:?}, error message: {error_message}, error data: {error_data:?}"
    )]
    ClientRequest {
        status_code: u16,
        error_code: Option<u16>,
        error_message: String,
        error_data: Option<String>,
    },
    #[error("Server error: status code: {status_code}, error message: {error_message}")]
    ServerRequest {
        status_code: u16,
        error_message: String,
    },
    #[error("Generic request error: {0}")]
    GenericRequest(String),
    #[error("Json parse error: {0}")]
    JsonParse(String),
    #[error("Serialization error: {0}")]
    SerializationFailure(String),
    #[error("Parse error: {0}")]
    GenericParse(String),
    #[error("Rmp parse error: {0}")]
    RmpParse(String),
    #[error("ECDSA signature failed: {0}")]
    SignatureFailure(String),
    #[error("ABI encoding error: {rust_type} cannot be encoded as EIP-712 type {abi_type}")]
    AbiEncode {
        rust_type: &'static str,
        abi_type: String,
    },
    /// WebSocket connection failed.
    #[error("WebSocket connect error: {0}")]
    WsConnect(String),
    /// WebSocket send failed.
    #[error("WebSocket send error: {0}")]
    WsSend(String),
    /// WebSocket receive failed.
    #[error("WebSocket receive error: {0}")]
    WsReceive(String),
}

#[derive(Error, Debug, Clone)]
pub enum ApiError {
    #[error(
        "Insufficient staked HYPE: {message}. You need to stake HYPE tokens to deploy perp assets."
    )]
    InsufficientStakedHype { message: String },
    // #[error(
    //     "Signature verification failed: {message}. This usually indicates a mismatch between the signed data and what the server expects. Check that you're using the correct wallet and network."
    // )]
    // When Hyperliquid computes a different signature compared to what you provided it answers "User or API Wallet 0x… does not exist" (with a different value every time). Most likely because the p and s are invalid, they don’t follow Hyperliquid’s formatting rules.
    // SignatureMismatch { message: String },

    // #[error(
    //     "User or API wallet not found: {address}. This may indicate a signature mismatch - the server recovered a different address from your signature than expected."
    // )]
    // WalletNotFound { address: String },
    #[error("Exchange API error: {message}")]
    Other { message: String },
}
