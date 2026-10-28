use hl_rs_derive::L1Action;
use rust_decimal::Decimal;
use serde::{
    de::{self, MapAccess, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::fmt;

/// Serialize Decimal as normalized string for HL wire format (matches Python float_to_wire).
fn serialize_decimal_wire<S>(d: &Decimal, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let s = d.normalize().to_string();
    serializer.serialize_str(&s)
}

/// Deserialize Decimal from string.
fn deserialize_decimal_wire<'de, D>(deserializer: D) -> Result<Decimal, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;
    let s = String::deserialize(deserializer)?;
    s.trim()
        .parse::<Decimal>()
        .map_err(D::Error::custom)
        .map(|d| d.normalize())
}

/// Order type for limit orders with time-in-force.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LimitOrderType {
    pub tif: Tif,
}

impl LimitOrderType {
    pub const fn gtc() -> Self {
        Self { tif: Tif::Gtc }
    }

    /// Immediate-or-cancel — use with a limit price at/through the book for market-style fills.
    pub const fn ioc() -> Self {
        Self { tif: Tif::Ioc }
    }

    /// Add-liquidity-only (post-only) — canceled instead of taking liquidity.
    pub const fn alo() -> Self {
        Self { tif: Tif::Alo }
    }
}

/// Time-in-force options.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tif {
    /// Good-til-cancelled
    Gtc,
    /// Immediate-or-cancel
    Ioc,
    /// Add liquidity only (post-only): the order is canceled instead of
    /// immediately matching against resting liquidity.
    Alo,
    /// Front-running protection
    FrontendMarket,
}

/// Trigger order type for stop-loss/take-profit.
///
/// **Field order is the wire format, not a style choice.** The L1 signing hash is taken over the
/// msgpack of this struct and msgpack key order follows declaration order, so the server recomputes
/// a different hash if these are declared in a different order than it expects — and the failure is
/// not a rejected signature. It recovers a different address and answers "User or API Wallet 0x…
/// does not exist", for an account nobody owns.
///
/// Python's `order_type_to_wire` builds the trigger dict as `isMarket`, `triggerPx`, `tpsl`, in that
/// order, so that is the order here. Note it differs from the order in Python's own
/// `TriggerOrderType` TypedDict, which is why reading the type rather than the wire builder gets
/// this wrong.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TriggerOrderType {
    pub is_market: bool,
    /// A normalized string, matching Python's `float_to_wire`.
    #[serde(
        serialize_with = "serialize_decimal_wire",
        deserialize_with = "deserialize_decimal_wire"
    )]
    pub trigger_px: Decimal,
    pub tpsl: TpSl,
}

/// Take-profit or stop-loss indicator.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TpSl {
    Tp,
    Sl,
}

/// Order type (limit or trigger).
/// API expects `{"limit": {"tif": "Gtc"}}` or `{"trigger": {...}}`, not untagged inner struct.
#[derive(Debug, Clone)]
pub enum OrderType {
    Limit(LimitOrderType),
    Trigger(TriggerOrderType),
}

impl Serialize for OrderType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(1))?;
        match self {
            OrderType::Limit(inner) => map.serialize_entry("limit", inner)?,
            OrderType::Trigger(inner) => map.serialize_entry("trigger", inner)?,
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for OrderType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct OrderTypeVisitor;
        impl<'de> Visitor<'de> for OrderTypeVisitor {
            type Value = OrderType;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("order type object with \"limit\" or \"trigger\" key")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut limit = None;
                let mut trigger = None;
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "limit" => limit = Some(map.next_value()?),
                        "trigger" => trigger = Some(map.next_value()?),
                        _ => {
                            let _: serde::de::IgnoredAny = map.next_value()?;
                        }
                    }
                }
                if let Some(inner) = limit {
                    Ok(OrderType::Limit(inner))
                } else if let Some(inner) = trigger {
                    Ok(OrderType::Trigger(inner))
                } else {
                    Err(de::Error::custom(
                        "order type must have \"limit\" or \"trigger\" key",
                    ))
                }
            }
        }
        deserializer.deserialize_map(OrderTypeVisitor)
    }
}

impl OrderType {
    pub fn limit(tif: Tif) -> Self {
        Self::Limit(LimitOrderType { tif })
    }

    pub fn limit_gtc() -> Self {
        Self::Limit(LimitOrderType::gtc())
    }

    /// IOC limit — Hyperliquid has no separate market type; cross at `limit_px` immediately.
    pub fn limit_ioc() -> Self {
        Self::Limit(LimitOrderType::ioc())
    }

    pub fn trigger(trigger_px: Decimal, is_market: bool, tpsl: TpSl) -> Self {
        Self::Trigger(TriggerOrderType {
            trigger_px,
            is_market,
            tpsl,
        })
    }
}

/// Wire format for a single order.
///
/// Field names match the Hyperliquid JSON/msgpack keys (`a`, `b`, …) via `rename`.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OrderWire {
    /// Asset index (wire `a`)
    #[serde(rename = "a")]
    pub asset: u32,
    /// Buy side (wire `b`)
    #[serde(rename = "b")]
    pub is_buy: bool,
    /// Limit price (wire `p`)
    #[serde(
        rename = "p",
        serialize_with = "serialize_decimal_wire",
        deserialize_with = "deserialize_decimal_wire"
    )]
    pub limit_px: Decimal,
    /// Size (wire `s`)
    #[serde(
        rename = "s",
        serialize_with = "serialize_decimal_wire",
        deserialize_with = "deserialize_decimal_wire"
    )]
    pub size: Decimal,
    /// Reduce only (wire `r`)
    #[serde(rename = "r")]
    pub reduce_only: bool,
    /// Limit or trigger payload (wire `t`)
    #[serde(rename = "t")]
    pub order_type: OrderType,
    /// Client order ID (wire `c`)
    #[serde(rename = "c", skip_serializing_if = "Option::is_none")]
    pub client_order_id: Option<String>,
}

/// Builder info for order attribution.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BuilderInfo {
    /// Builder address (lowercase)
    pub b: String,
    /// Builder fee in basis points
    pub f: u32,
}

/// Order grouping options.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum Grouping {
    #[default]
    Na,
    /// Vault orders
    NormalTpsl,
    /// Position TP/SL
    PositionTpsl,
}

/// Batch create orders action.
/// Uses payload_key = "order" (same as action_type). L1 signing prepends `"type": "order"`
/// via [`L1ActionWrapper`](crate::actions::L1ActionWrapper); this impl must omit `type`
/// to avoid duplicate map keys in msgpack (must match Python `msgpack.packb(action)`).
///
/// Key order for hashing: type (from wrapper), orders, grouping, builder.
#[derive(Deserialize, Debug, Clone, L1Action)]
#[action(action_type = "order", payload_key = "order")]
#[serde(rename_all = "camelCase")]
pub struct BatchOrder {
    /// Order wires
    pub orders: Vec<OrderWire>,
    /// Grouping type
    pub grouping: Grouping,
    /// Builder info (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub builder: Option<BuilderInfo>,
    #[serde(skip_serializing)]
    pub nonce: Option<u64>,
}

impl Serialize for BatchOrder {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeMap;
        let len = 2 + self.builder.is_some() as usize;
        let mut map = serializer.serialize_map(Some(len))?;
        map.serialize_entry("orders", &self.orders)?;
        map.serialize_entry("grouping", &self.grouping)?;
        if let Some(ref b) = self.builder {
            map.serialize_entry("builder", b)?;
        }
        map.end()
    }
}

impl BatchOrder {
    pub fn new(orders: Vec<OrderWire>) -> Self {
        Self {
            orders,
            grouping: Grouping::Na,
            builder: None,
            nonce: None,
        }
    }

    pub fn with_grouping(mut self, grouping: Grouping) -> Self {
        self.grouping = grouping;
        self
    }

    pub fn with_builder(mut self, builder: BuilderInfo) -> Self {
        self.builder = Some(builder);
        self
    }
}

#[cfg(test)]
mod trigger_wire_order {
    use rust_decimal::Decimal;

    use super::{LimitOrderType, OrderType, TpSl, TriggerOrderType};

    fn keys(v: &serde_json::Value) -> Vec<String> {
        v.as_object()
            .expect("an object")
            .keys()
            .cloned()
            .collect()
    }

    /// Pins the order Python's `order_type_to_wire` emits: `isMarket`, `triggerPx`, `tpsl`.
    ///
    /// This is the whole bug this module's doc comment describes. The hash is taken over msgpack
    /// whose key order follows declaration order, so a field moving here produces a signature the
    /// server cannot match — and it reports an unknown user rather than a bad signature, which is
    /// why it cost a live debugging session to find.
    #[test]
    fn the_trigger_dict_is_is_market_then_trigger_px_then_tpsl() {
        let t = OrderType::Trigger(TriggerOrderType {
            is_market: false,
            trigger_px: Decimal::new(50, 0),
            tpsl: TpSl::Sl,
        });
        let v = serde_json::to_value(&t).unwrap();
        assert_eq!(keys(&v), vec!["trigger"]);
        assert_eq!(
            keys(&v["trigger"]),
            vec!["isMarket", "triggerPx", "tpsl"],
            "must match Python's order_type_to_wire, not its TypedDict"
        );
    }

    /// msgpack is what the hash actually covers, so assert the order there too rather than
    /// trusting serde_json's object to preserve it.
    #[test]
    fn msgpack_carries_the_keys_in_that_same_order() {
        let t = OrderType::Trigger(TriggerOrderType {
            is_market: true,
            trigger_px: Decimal::new(50, 0),
            tpsl: TpSl::Tp,
        });
        let hex: String = rmp_serde::to_vec_named(&t)
            .unwrap()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let at =
            |needle: &str| hex.find(needle).unwrap_or_else(|| panic!("missing {needle} in {hex}"));
        // fixstr-prefixed keys: a8 "isMarket", a9 "triggerPx", a4 "tpsl".
        let is_market = at("a869734d61726b6574");
        let trigger_px = at("a9747269676765725078");
        let tpsl = at("a47470736c");
        assert!(is_market < trigger_px, "isMarket must precede triggerPx: {hex}");
        assert!(trigger_px < tpsl, "triggerPx must precede tpsl: {hex}");
    }

    /// The limit arm is the control: it was never broken, and must stay that way.
    #[test]
    fn the_limit_arm_is_unchanged() {
        let v = serde_json::to_value(OrderType::Limit(LimitOrderType::alo())).unwrap();
        assert_eq!(keys(&v), vec!["limit"]);
        assert_eq!(keys(&v["limit"]), vec!["tif"]);
        assert_eq!(v["limit"]["tif"], serde_json::json!("Alo"));
    }
}

#[cfg(test)]
mod grouping_wire {
    use super::Grouping;

    /// Python's `Grouping` is `"na" | "normalTpsl" | "positionTpsl"`.
    #[test]
    fn grouping_serializes_as_camel_case() {
        assert_eq!(serde_json::to_value(Grouping::Na).unwrap(), "na");
        assert_eq!(
            serde_json::to_value(Grouping::NormalTpsl).unwrap(),
            "normalTpsl"
        );
        assert_eq!(
            serde_json::to_value(Grouping::PositionTpsl).unwrap(),
            "positionTpsl"
        );
    }
}
