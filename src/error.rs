use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum Error {
    #[error(transparent)]
    Api(#[from] ApiError),

    // TODO: turn some embedded types into errors instead of strings
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
    #[error("Generic request error: {0:?}")]
    GenericRequest(String),
    #[error("Chain type not allowed for this function")]
    ChainNotAllowed,
    #[error("Asset not found")]
    AssetNotFound,
    #[error("Error from Eip712 struct: {0:?}")]
    Eip712(String),
    #[error("Json parse error: {0:?}")]
    JsonParse(String),
    #[error("Generic parse error: {0:?}")]
    GenericParse(String),
    #[error("Wallet error: {0:?}")]
    Wallet(String),
    #[error("Websocket error: {0:?}")]
    Websocket(String),
    #[error("Subscription not found")]
    SubscriptionNotFound,
    #[error("WS manager not instantiated")]
    WsManagerNotFound,
    #[error("WS send error: {0:?}")]
    WsSend(String),
    #[error("Reader data not found")]
    ReaderDataNotFound,
    #[error("Reader error: {0:?}")]
    GenericReader(String),
    #[error("Reader text conversion error: {0:?}")]
    ReaderTextConversion(String),
    #[error("Order type not found")]
    OrderTypeNotFound,
    #[error("Issue with generating random data: {0:?}")]
    RandGen(String),
    #[error("Private key parse error: {0:?}")]
    PrivateKeyParse(String),
    #[error("Cannot subscribe to multiple user events")]
    UserEvents,
    #[error("Rmp parse error: {0:?}")]
    RmpParse(String),
    #[error("Invalid input number")]
    FloatStringParse,
    #[error("No cloid found in order request when expected")]
    NoCloid,
    #[error("ECDSA signature failed: {0:?}")]
    SignatureFailure(String),
    #[error("Error recovering address from signature: {0:?}")]
    RecoverAddressFailure(String),
    #[error("Vault address not found")]
    VaultAddressNotFound,
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
