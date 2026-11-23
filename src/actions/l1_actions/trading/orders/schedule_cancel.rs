use hl_rs_derive::L1Action;
use serde::{Deserialize, Serialize};

/// Schedule a cancel-all action at a future time: a dead-man's switch.
///
/// Time must be at least 5 seconds in the future.
/// Maximum 10 scheduled cancels per day.
///
/// **`time: None` unsets the schedule; it does not cancel anything.** The SDK documents its
/// `schedule_cancel(None)` as "unsets any cancel time in the future", and sends the action with no
/// `time` key. A caller building a dead-man's switch who reached for an "immediately" constructor
/// would disarm the switch instead of firing it, so the constructor for that case is
/// [`ScheduleCancel::unset`].
#[derive(Serialize, Deserialize, Debug, Clone, L1Action)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleCancel {
    /// UTC milliseconds at which to cancel all open orders, or `None` to unset any scheduled
    /// cancel. `None` is the absent key: it recovers the signer and `"time":null` does not (testnet
    /// 2026-11-20: signed by a key approved nowhere, the answer named that key).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<u64>,
    #[serde(skip_serializing)]
    pub nonce: Option<u64>,
}

impl ScheduleCancel {
    /// Schedule a cancel at a specific time (UTC milliseconds).
    pub fn at(time: u64) -> Self {
        Self {
            time: Some(time),
            nonce: None,
        }
    }

    /// Unset any scheduled cancel. Cancels nothing.
    pub fn unset() -> Self {
        Self {
            time: None,
            nonce: None,
        }
    }

    /// Misnamed: this **unsets** the schedule, as [`ScheduleCancel::unset`] does. It never
    /// cancelled immediately.
    #[deprecated(note = "unsets the scheduled cancel rather than cancelling now; use `unset()`")]
    pub fn now() -> Self {
        Self::unset()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::L1ActionWrapper;

    fn msgpack_hex(a: &ScheduleCancel) -> String {
        alloy::hex::encode(rmp_serde::to_vec_named(&L1ActionWrapper { action: a }).unwrap())
    }

    /// `Exchange.schedule_cancel(None)` in SDK 0.21.0, through CPython `msgpack.packb`: the type
    /// alone, no `time` key.
    #[test]
    fn unset_is_the_type_alone() {
        assert_eq!(
            msgpack_hex(&ScheduleCancel::unset()),
            "81a474797065ae7363686564756c6543616e63656c"
        );
    }

    /// `Exchange.schedule_cancel(1795182554858)`.
    #[test]
    fn at_carries_the_time() {
        assert_eq!(
            msgpack_hex(&ScheduleCancel::at(1_795_182_554_858)),
            "82a474797065ae7363686564756c6543616e63656ca474696d65cf000001a1f937eeea"
        );
    }

    #[test]
    #[allow(deprecated)]
    fn now_is_unset() {
        assert_eq!(ScheduleCancel::now().time, ScheduleCancel::unset().time);
        assert_eq!(ScheduleCancel::now().time, None);
    }
}
