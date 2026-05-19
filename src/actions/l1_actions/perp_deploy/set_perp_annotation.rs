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
/// assert!(action.keywords.is_empty());
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
///     .display_name("BTC Perp")
///     .keywords(["perp", "crypto"])
///     .build()
///     .unwrap();
/// assert_eq!(action.coin, "mydex:BTC");
/// assert_eq!(action.display_name.as_deref(), Some("BTC Perp"));
/// assert_eq!(action.keywords, vec!["perp".to_string(), "crypto".to_string()]);
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
    /// Optional display name for the asset ([HIP-3 docs](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/hip-3-deployer-actions)).
    pub display_name: Option<String>,
    /// Search / filter keywords for the asset ([HIP-3 docs](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/hip-3-deployer-actions)).
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(skip_serializing)]
    pub nonce: Option<u64>,
}

/// Builder for [`SetPerpAnnotation`].
///
/// String setters take `impl Into<String>`; use [`keywords`](SetPerpAnnotationBuilder::keywords) for
/// the keyword list. [`build`](SetPerpAnnotationBuilder::build) returns
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
///     .display_name("ETH")
///     .keywords(Vec::<String>::new())
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
    display_name: Option<String>,
    keywords: Vec<String>,
}

impl SetPerpAnnotationBuilder {
    /// Start building with required dex and coin.
    pub fn new(dex: impl Into<String>, coin: impl Into<String>) -> Self {
        Self {
            dex: Some(dex.into()),
            coin: Some(coin.into()),
            category: String::new(),
            description: String::new(),
            display_name: None,
            keywords: Vec::new(),
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

    /// Set the display name (optional).
    pub fn display_name(mut self, display_name: impl Into<String>) -> Self {
        self.display_name = Some(display_name.into());
        self
    }

    /// Set the full keyword list (replaces any previous keywords).
    pub fn keywords(mut self, keywords: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.keywords = keywords.into_iter().map(Into::into).collect();
        self
    }

    /// Append a single keyword.
    pub fn push_keyword(mut self, keyword: impl Into<String>) -> Self {
        self.keywords.push(keyword.into());
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
            display_name: self.display_name,
            keywords: self.keywords,
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
            display_name: None,
            keywords: Vec::new(),
            nonce: None,
        }
    }

    /// Start a builder with required dex and coin (both take `impl Into<String>`).
    pub fn builder(dex: impl Into<String>, coin: impl Into<String>) -> SetPerpAnnotationBuilder {
        SetPerpAnnotationBuilder::new(dex, coin)
    }
}
