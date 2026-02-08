use crate::flatten_vec;
use hl_rs_derive::L1Action;

/// Set funding interest rates for assets in a perp DEX.
///
/// Interest rates must be in the range [-0.01, 0.01] and are used as the
/// interest rate component in the funding calculation. This represents
/// the 8-hour interest rate.
///
/// The tuples are sorted by asset name during serialization.
///
/// # Examples
///
/// Set interest rates for multiple assets:
///
/// ```
/// use hl_rs::actions::SetFundingInterestRates;
///
/// let action = SetFundingInterestRates::new("mydex", vec![
///     ("BTC", 0.0001),   // 0.01% 8-hour rate
///     ("ETH", -0.0001),  // -0.01% 8-hour rate
/// ]);
///
/// assert_eq!(action.rates.len(), 2);
/// assert_eq!(action.rates[0].0, "mydex:BTC");
/// ```
#[derive(Debug, Clone, L1Action)]
#[action(action_type = "perpDeploy", payload_key = "setFundingInterestRates")]
pub struct SetFundingInterestRates {
    /// Vec of (asset, interest_rate) tuples.
    /// Interest rates must be between -0.01 and 0.01.
    pub rates: Vec<(String, String)>,
    pub nonce: Option<u64>,
}

impl SetFundingInterestRates {
    /// Create a new SetFundingInterestRates action.
    ///
    /// # Arguments
    /// * `dex_name` - Name of the perp DEX
    /// * `rates` - Vec of (asset, interest_rate) tuples.
    ///             Interest rates must be in range [-0.01, 0.01].
    ///
    /// # Example
    ///
    /// ```
    /// use hl_rs::actions::SetFundingInterestRates;
    ///
    /// let action = SetFundingInterestRates::new("mydex", vec![
    ///     ("BTC", 0.0001),
    ///     ("ETH", 0.0002),
    /// ]);
    /// ```
    pub fn new(dex_name: impl Into<String>, rates: Vec<(impl Into<String>, f64)>) -> Self {
        let dex_name = dex_name.into().to_lowercase();
        Self {
            rates: rates
                .into_iter()
                .map(|(asset, rate)| {
                    (
                        format!("{dex_name}:{}", asset.into().to_uppercase()),
                        rate.to_string(),
                    )
                })
                .collect(),
            nonce: None,
        }
    }
}

flatten_vec!(SetFundingInterestRates, rates);
