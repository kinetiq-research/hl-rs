use crate::flatten_vec;
use hl_rs_derive::L1Action;
use rust_decimal::Decimal;

/// Set 8-hour funding clamps for assets in a perp DEX.
///
/// Clamps must be in the range [0, 0.01] and bound how far the funding rate
/// can move away from the average premium toward the interest rate.
/// Defaults to 0.0003.
///
/// The tuples are sorted by asset name during serialization.
///
/// # Examples
///
/// ```
/// use hl_rs::actions::SetFundingClamps;
/// use rust_decimal_macros::dec;
///
/// let action = SetFundingClamps::new("mydex", vec![
///     ("BTC", dec!(0.0003)),
///     ("ETH", dec!(0.001)),
/// ]);
///
/// assert_eq!(action.clamps.len(), 2);
/// assert_eq!(action.clamps[0].0, "mydex:BTC");
/// ```
#[derive(Debug, Clone, L1Action)]
#[action(action_type = "perpDeploy", payload_key = "setFundingClamps")]
pub struct SetFundingClamps {
    /// Vec of (asset, clamp) tuples.
    /// Clamps must be between 0 and 0.01.
    pub clamps: Vec<(String, String)>,
    pub nonce: Option<u64>,
}

impl SetFundingClamps {
    /// Create a new SetFundingClamps action.
    ///
    /// # Arguments
    /// * `dex_name` - Name of the perp DEX
    /// * `clamps` - Vec of (asset, clamp) tuples. Clamps must be in range [0, 0.01].
    pub fn new(dex_name: impl Into<String>, clamps: Vec<(impl Into<String>, Decimal)>) -> Self {
        let dex_name = dex_name.into().to_lowercase();
        Self {
            clamps: clamps
                .into_iter()
                .map(|(asset, clamp)| {
                    (
                        format!("{dex_name}:{}", asset.into().to_uppercase()),
                        clamp.to_string(),
                    )
                })
                .collect(),
            nonce: None,
        }
    }
}

flatten_vec!(SetFundingClamps, clamps);

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn serializes_sorted_tuples() {
        let action =
            SetFundingClamps::new("mydex", vec![("eth", dec!(0.001)), ("btc", dec!(0.0003))]);
        let json = serde_json::to_string(&action).unwrap();
        assert_eq!(json, r#"[["mydex:BTC","0.0003"],["mydex:ETH","0.001"]]"#);
    }
}
