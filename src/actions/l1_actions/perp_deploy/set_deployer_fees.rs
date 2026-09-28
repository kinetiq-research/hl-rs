use crate::flatten_vec;
use hl_rs_derive::L1Action;
use serde::{Deserialize, Serialize};

/// Deployer fee configuration for a single asset.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub struct DeployerFeeConfig {
    /// Fee scale as a decimal string in `[0.0, 3.0]`, or `[0.0, 10.0)` when
    /// `growth_mode` is true.
    pub scale: String,
    /// When true, protocol-side fees, rebates, and volume contributions are reduced by 90%.
    pub growth_mode: bool,
}

impl DeployerFeeConfig {
    pub fn new(scale: impl Into<String>, growth_mode: bool) -> Self {
        Self {
            scale: scale.into(),
            growth_mode,
        }
    }
}

/// Set deployer fee scale and growth mode per asset on a perp DEX.
///
/// Fee scale controls the deployer's share of user trading fees. When growth mode is
/// enabled, the allowed scale range expands to `[0.0, 10.0)`.
///
/// On mainnet, changes are rate limited to one per 30 days per asset. See
/// [HIP-3 deployer actions](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/hip-3-deployer-actions).
///
/// # Examples
///
/// Set deployer fees for multiple assets:
///
/// ```
/// use hl_rs::actions::{DeployerFeeConfig, PerpSetDeployerFees};
///
/// let action = PerpSetDeployerFees::new("mydex", vec![
///     ("BTC", DeployerFeeConfig::new("1.0", false)),
///     ("ETH", DeployerFeeConfig::new("0.5", true)),
/// ]);
///
/// assert_eq!(action.fees.len(), 2);
/// assert_eq!(action.fees[0].0, "mydex:BTC");
/// ```
#[derive(Debug, Clone, L1Action, Default)]
#[action(action_type = "perpDeploy", payload_key = "setDeployerFees")]
pub struct PerpSetDeployerFees {
    /// List of (asset, fee config) tuples. Asset format is `"dex:SYMBOL"`.
    pub fees: Vec<(String, DeployerFeeConfig)>,
    pub nonce: Option<u64>,
}

impl PerpSetDeployerFees {
    /// Create a new `PerpSetDeployerFees` action.
    ///
    /// # Arguments
    ///
    /// * `dex` - The DEX name (will be lowercased)
    /// * `fees` - Vector of (asset symbol, fee config) tuples
    pub fn new(dex: impl Into<String>, fees: Vec<(impl Into<String>, DeployerFeeConfig)>) -> Self {
        let dex = dex.into().to_lowercase();
        Self {
            fees: fees
                .into_iter()
                .map(|(asset, config)| (format!("{}:{}", dex, asset.into().to_uppercase()), config))
                .collect(),
            nonce: None,
        }
    }

    /// Create a `PerpSetDeployerFees` action for a single asset.
    pub fn set_single(
        dex: impl Into<String>,
        asset: impl Into<String>,
        scale: impl Into<String>,
        growth_mode: bool,
    ) -> Self {
        Self::default().set_fee(dex, asset, DeployerFeeConfig::new(scale, growth_mode))
    }

    /// Add a fee config for an asset (builder pattern).
    pub fn set_fee(
        mut self,
        dex: impl Into<String>,
        asset: impl Into<String>,
        config: DeployerFeeConfig,
    ) -> Self {
        self.fees.push((
            format!(
                "{}:{}",
                dex.into().to_lowercase(),
                asset.into().to_uppercase()
            ),
            config,
        ));
        self
    }
}

flatten_vec!(PerpSetDeployerFees, fees);
