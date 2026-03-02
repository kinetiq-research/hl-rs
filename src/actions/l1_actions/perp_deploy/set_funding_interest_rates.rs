use crate::flatten_vec;
use hl_rs_derive::L1Action;
use rust_decimal::Decimal;

/// Set funding interest rates for assets in a perp DEX.
///
/// Interest rates must be in the range [-0.01, 0.01] and are used as the
/// interest rate component in the funding calculation. This represents
/// the 8-hour interest rate.
///
/// **NOTE:** This is the baseline interest rate that will be displayed if
/// premium is 0 and funding scaling is 1.
///
/// The tuples are sorted by asset name during serialization.
///
/// # Examples
///
/// Set interest rates for multiple assets:
///
/// ```
/// use hl_rs::actions::SetFundingInterestRates;
/// use rust_decimal_macros::dec;
///
/// let action = SetFundingInterestRates::new("mydex", vec![
///     ("BTC", dec!(0.0001)),   // 0.01% 8-hour rate
///     ("ETH", dec!(-0.0001)),  // -0.01% 8-hour rate
/// ]);
///
/// assert_eq!(action.rates.len(), 2);
/// assert_eq!(action.rates[0].0, "mydex:BTC");
/// ```
///
/// Set rate using target APY:
///
/// ```
/// use hl_rs::actions::SetFundingInterestRates;
///
/// // Set a 5% APY baseline interest rate for BTC
/// let action = SetFundingInterestRates::from_target_apy("mydex", "BTC", 5.0);
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
    /// **NOTE:** This is the baseline interest rate that will be displayed if
    /// premium is 0 and funding scaling is 1.
    ///
    /// # Arguments
    /// * `dex_name` - Name of the perp DEX
    /// * `rates` - Vec of (asset, interest_rate) tuples.
    ///   Interest rates must be in range [-0.01, 0.01].
    ///
    /// # Example
    ///
    /// ```
    /// use hl_rs::actions::SetFundingInterestRates;
    /// use rust_decimal_macros::dec;
    ///
    /// let action = SetFundingInterestRates::new("mydex", vec![
    ///     ("BTC", dec!(0.0001)),
    ///     ("ETH", dec!(0.0002)),
    /// ]);
    /// ```
    pub fn new(dex_name: impl Into<String>, rates: Vec<(impl Into<String>, Decimal)>) -> Self {
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

    /// Create a SetFundingInterestRates action from a target APY.
    ///
    /// This is a convenience method that converts an annual percentage yield (APY)
    /// to the 8-hour interest rate expected by Hyperliquid.
    ///
    /// **NOTE:** This is the baseline interest rate that will be displayed if
    /// premium is 0 and funding scaling is 1.
    ///
    /// # Arguments
    /// * `dex_name` - Name of the perp DEX
    /// * `coin` - The coin symbol (will be uppercased)
    /// * `target_apy` - The target annual percentage yield (e.g., 5.0 for 5% APY)
    ///
    /// # Example
    ///
    /// ```
    /// use hl_rs::actions::SetFundingInterestRates;
    ///
    /// // Set a 5% APY baseline interest rate for BTC
    /// let action = SetFundingInterestRates::from_target_apy("mydex", "BTC", 5.0);
    /// assert_eq!(action.rates.len(), 1);
    /// ```
    pub fn from_target_apy(
        dex_name: impl Into<String>,
        coin: impl Into<String>,
        target_apy: f64,
    ) -> Self {
        let dex_name = dex_name.into().to_lowercase();
        let coin = coin.into().to_uppercase();

        // Convert APY to 8-hour rate
        // periods = (365 days * 24 hours) / 8 hours = 1095 periods per year
        let periods = (365.0 * 24.0) / 8.0;
        let rate = (target_apy * 0.01) / periods;

        Self {
            rates: vec![(format!("{dex_name}:{coin}"), rate.to_string())],
            nonce: None,
        }
    }
}

flatten_vec!(SetFundingInterestRates, rates);
