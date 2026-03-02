use hl_rs_derive::L1Action;
use serde::{Deserialize, Serialize};

/// Error returned when building [`SetPerpAnnotation`] with missing required fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetPerpAnnotationBuildError {
    /// The DEX name was not set.
    MissingDex,
    /// The coin symbol was not set.
    MissingCoin,
}

impl std::fmt::Display for SetPerpAnnotationBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingDex => f.write_str("dex is required to build SetPerpAnnotation"),
            Self::MissingCoin => f.write_str("coin is required to build SetPerpAnnotation"),
        }
    }
}

impl std::error::Error for SetPerpAnnotationBuildError {}

/// Set the category and description annotation for a perp asset on a DEX.
///
/// This action allows DEX deployers to set or update the category and
/// description metadata displayed for a perpetual asset.
///
/// # Examples
///
/// ```
/// use hl_rs::actions::SetPerpAnnotation;
///
/// let action = SetPerpAnnotation::new("mydex", "BTC", "Crypto", "Bitcoin perpetual");
/// assert_eq!(action.coin, "mydex:BTC");
/// assert_eq!(action.category, "Crypto");
/// assert_eq!(action.description, "Bitcoin perpetual");
/// ```
///
/// Using the builder:
///
/// ```
/// use hl_rs::actions::SetPerpAnnotation;
///
/// let action = SetPerpAnnotation::builder("mydex", "BTC")
///     .category("Crypto")
///     .description("Bitcoin perpetual")
///     .build()
///     .unwrap();
/// assert_eq!(action.coin, "mydex:BTC");
/// ```
#[derive(Serialize, Deserialize, Debug, Clone, L1Action)]
#[action(action_type = "perpDeploy", payload_key = "setPerpAnnotation")]
#[serde(rename_all = "camelCase")]
pub struct SetPerpAnnotation {
    /// The coin identifier in the format "dex:SYMBOL" (e.g., "mydex:BTC").
    pub coin: String,
    /// Category label for the asset.
    pub category: String,
    /// Description text for the asset.
    pub description: String,
    #[serde(skip_serializing)]
    pub nonce: Option<u64>,
}

/// Builder for [`SetPerpAnnotation`].
///
/// All setter methods take `impl Into<String>`. [`build`](SetPerpAnnotationBuilder::build) returns
/// an error if `dex` or `coin` were not set.
///
/// # Example
///
/// ```
/// use hl_rs::actions::SetPerpAnnotation;
///
/// let action = SetPerpAnnotation::builder("mydex", "ETH")
///     .category("Crypto")
///     .description("Ethereum perpetual")
///     .build()
///     .unwrap();
/// ```
///
/// Missing required fields yield an error:
///
/// ```
/// use hl_rs::actions::{SetPerpAnnotationBuilder, SetPerpAnnotationBuildError};
///
/// let err = SetPerpAnnotationBuilder::default()
///     .coin("BTC")
///     .build()
///     .unwrap_err();
/// assert_eq!(err, SetPerpAnnotationBuildError::MissingDex);
/// ```
#[derive(Debug, Default, Clone)]
pub struct SetPerpAnnotationBuilder {
    dex: Option<String>,
    coin: Option<String>,
    category: String,
    description: String,
}

impl SetPerpAnnotationBuilder {
    /// Start building with required dex and coin.
    pub fn new(dex: impl Into<String>, coin: impl Into<String>) -> Self {
        Self {
            dex: Some(dex.into()),
            coin: Some(coin.into()),
            category: String::new(),
            description: String::new(),
        }
    }

    /// Set the DEX name (required).
    pub fn dex(mut self, dex: impl Into<String>) -> Self {
        self.dex = Some(dex.into());
        self
    }

    /// Set the coin symbol (required).
    pub fn coin(mut self, coin: impl Into<String>) -> Self {
        self.coin = Some(coin.into());
        self
    }

    /// Set the category.
    pub fn category(mut self, category: impl Into<String>) -> Self {
        self.category = category.into();
        self
    }

    /// Set the description.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Build the action. Returns an error if `dex` or `coin` was not set.
    pub fn build(self) -> Result<SetPerpAnnotation, SetPerpAnnotationBuildError> {
        let dex = self.dex.ok_or(SetPerpAnnotationBuildError::MissingDex)?;
        let coin = self.coin.ok_or(SetPerpAnnotationBuildError::MissingCoin)?;
        Ok(SetPerpAnnotation {
            coin: format!("{}:{}", dex.to_lowercase(), coin.to_uppercase()),
            category: self.category,
            description: self.description,
            nonce: None,
        })
    }
}

impl SetPerpAnnotation {
    /// Create a new `SetPerpAnnotation` action.
    ///
    /// # Arguments
    ///
    /// * `dex` - The DEX name (will be lowercased)
    /// * `coin` - The coin symbol (will be uppercased)
    /// * `category` - Category label for the asset
    /// * `description` - Description text for the asset
    ///
    /// # Example
    ///
    /// ```
    /// use hl_rs::actions::SetPerpAnnotation;
    ///
    /// let action = SetPerpAnnotation::new("mydex", "BTC", "Crypto", "Bitcoin perpetual");
    /// assert_eq!(action.coin, "mydex:BTC");
    /// ```
    pub fn new(
        dex: impl Into<String>,
        coin: impl Into<String>,
        category: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            coin: format!(
                "{}:{}",
                dex.into().to_lowercase(),
                coin.into().to_uppercase()
            ),
            category: category.into(),
            description: description.into(),
            nonce: None,
        }
    }

    /// Start a builder with required dex and coin (both take `impl Into<String>`).
    pub fn builder(dex: impl Into<String>, coin: impl Into<String>) -> SetPerpAnnotationBuilder {
        SetPerpAnnotationBuilder::new(dex, coin)
    }
}
