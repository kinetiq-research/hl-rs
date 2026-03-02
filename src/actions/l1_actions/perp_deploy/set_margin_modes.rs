use crate::flatten_vec;
use hl_rs_derive::L1Action;
use serde::{Deserialize, Serialize};

/// Margin mode for perp assets.
///
/// See [RegisterAssetRequest2](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/hip-3-deployer-actions) for definitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MarginMode {
    /// Strict isolated: does not allow withdrawing isolated margin from open positions.
    StrictIsolated,
    /// No cross margin.
    NoCross,

    /// Normal
    Normal,
}

/// Set margin modes for assets in a perp DEX.
///
/// A sorted list of (coin, marginMode). The tuples are sorted by (coin, margin mode) during serialization.
///
/// # Examples
///
/// Set margin modes for multiple assets:
///
/// ```
/// use hl_rs::actions::{SetMarginModes, MarginMode};
///
/// let action = SetMarginModes::new("mydex", vec![
///     ("BTC", MarginMode::StrictIsolated),
///     ("ETH", MarginMode::NoCross),
/// ]);
///
/// assert_eq!(action.modes.len(), 2);
/// assert_eq!(action.modes[0].0, "mydex:BTC");
/// assert_eq!(action.modes[1].0, "mydex:ETH");
/// ```
#[derive(Debug, Clone, L1Action)]
#[action(action_type = "perpDeploy", payload_key = "setMarginModes")]
pub struct SetMarginModes {
    /// Vec of (asset, margin_mode) tuples
    pub modes: Vec<(String, MarginMode)>,
    pub nonce: Option<u64>,
}

impl SetMarginModes {
    /// Create a new SetMarginModes action.
    ///
    /// # Arguments
    /// * `dex_name` - Name of the perp DEX (used to prefix asset symbols)
    /// * `modes` - Vec of (asset, margin_mode) tuples
    ///
    /// # Example
    ///
    /// ```
    /// use hl_rs::actions::{SetMarginModes, MarginMode};
    ///
    /// let action = SetMarginModes::new("mydex", vec![
    ///     ("BTC", MarginMode::StrictIsolated),
    ///     ("ETH", MarginMode::NoCross),
    /// ]);
    /// ```
    pub fn new(dex_name: impl Into<String>, modes: Vec<(impl Into<String>, MarginMode)>) -> Self {
        let dex_name = dex_name.into().to_lowercase();
        Self {
            modes: modes
                .into_iter()
                .map(|(asset, mode)| (format!("{dex_name}:{}", asset.into().to_uppercase()), mode))
                .collect(),
            nonce: None,
        }
    }
}

flatten_vec!(SetMarginModes, modes);
