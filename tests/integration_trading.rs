//! Integration tests for trading actions.
//!
//! These tests require a funded testnet wallet.
//! Run with:
//! ```bash
//! HL_PRIVATE_KEY=0x... cargo test --features integration-tests integration_trading
//! ```

#![cfg(feature = "integration-tests")]

mod common;

use hl_rs::actions::{
    BatchCancel, BatchModify, BatchOrder, CancelByCloid, CancelByCloidWire, CancelWire, Grouping,
    LimitOrderType, ModifyWire, OrderType, OrderWire, ScheduleCancel, Tif, TpSl, TriggerOrderType,
    UpdateIsolatedMargin, UpdateLeverage,
};

use crate::common::{log_action, log_response, send_action};

/// Test asset index (ETH on testnet is typically 4)
const TEST_ASSET: u32 = 4;

// ============================================================================
// UpdateLeverage
// ============================================================================

#[tokio::test]
async fn test_update_leverage_cross() {
    let action = UpdateLeverage::cross(TEST_ASSET, 10);
    log_action("UpdateLeverage (cross)", &action);

    let result = send_action(action).await;
    log_response("UpdateLeverage (cross)", &result);
}

#[tokio::test]
async fn test_update_leverage_isolated() {
    let action = UpdateLeverage::isolated(TEST_ASSET, 5);
    log_action("UpdateLeverage (isolated)", &action);

    let result = send_action(action).await;
    log_response("UpdateLeverage (isolated)", &result);
}

// ============================================================================
// UpdateIsolatedMargin
// ============================================================================

#[tokio::test]
async fn test_update_isolated_margin_add() {
    let action = UpdateIsolatedMargin::add(TEST_ASSET, true, 1_000_000); // Add $1 to long
    log_action("UpdateIsolatedMargin (add)", &action);

    let result = send_action(action).await;
    log_response("UpdateIsolatedMargin (add)", &result);
}

#[tokio::test]
async fn test_update_isolated_margin_remove() {
    let action = UpdateIsolatedMargin::remove(TEST_ASSET, true, 500_000); // Remove $0.50 from long
    log_action("UpdateIsolatedMargin (remove)", &action);

    let result = send_action(action).await;
    log_response("UpdateIsolatedMargin (remove)", &result);
}

// ============================================================================
// BatchOrder
// ============================================================================

fn make_limit_order(asset: u32, is_buy: bool, price: &str, size: &str) -> OrderWire {
    OrderWire {
        asset,
        is_buy,
        limit_px: price.parse().unwrap(),
        size: size.parse().unwrap(),
        reduce_only: false,
        order_type: OrderType::Limit(LimitOrderType { tif: Tif::Gtc }),
        client_order_id: None,
    }
}

fn make_trigger_order(
    asset: u32,
    is_buy: bool,
    price: &str,
    size: &str,
    trigger_px: &str,
    tpsl: TpSl,
) -> OrderWire {
    OrderWire {
        asset,
        is_buy,
        limit_px: price.parse().unwrap(),
        size: size.parse().unwrap(),
        reduce_only: true, // Trigger orders are usually reduce-only
        order_type: OrderType::Trigger(TriggerOrderType {
            trigger_px: trigger_px.parse().unwrap(),
            is_market: true,
            tpsl,
        }),
        client_order_id: None,
    }
}

#[tokio::test]
async fn test_batch_order_single_limit() {
    let order = make_limit_order(TEST_ASSET, true, "100.0", "0.01");
    let action = BatchOrder::new(vec![order]);
    log_action("BatchOrder (single limit)", &action);

    let result = send_action(action).await;
    log_response("BatchOrder (single limit)", &result);
}

#[tokio::test]
async fn test_batch_order_multiple_limits() {
    let orders = vec![
        make_limit_order(TEST_ASSET, true, "100.0", "0.01"),
        make_limit_order(TEST_ASSET, true, "99.0", "0.01"),
        make_limit_order(TEST_ASSET, false, "110.0", "0.01"),
    ];
    let action = BatchOrder::new(orders);
    log_action("BatchOrder (multiple limits)", &action);

    let result = send_action(action).await;
    log_response("BatchOrder (multiple limits)", &result);
}

#[tokio::test]
async fn test_batch_order_with_cloid() {
    let mut order = make_limit_order(TEST_ASSET, true, "100.0", "0.01");
    order.client_order_id = Some("test-cloid-12345".to_string());

    let action = BatchOrder::new(vec![order]);
    log_action("BatchOrder (with cloid)", &action);

    let result = send_action(action).await;
    log_response("BatchOrder (with cloid)", &result);
}

#[tokio::test]
async fn test_batch_order_ioc() {
    let order = OrderWire {
        asset: TEST_ASSET,
        is_buy: true,
        limit_px: "100.0".parse().unwrap(),
        size: "0.01".parse().unwrap(),
        reduce_only: false,
        order_type: OrderType::Limit(LimitOrderType { tif: Tif::Ioc }),
        client_order_id: None,
    };
    let action = BatchOrder::new(vec![order]);
    log_action("BatchOrder (IOC)", &action);

    let result = send_action(action).await;
    log_response("BatchOrder (IOC)", &result);
}

#[tokio::test]
async fn test_batch_order_trigger_stop_loss() {
    let order = make_trigger_order(TEST_ASSET, false, "90.0", "0.01", "95.0", TpSl::Sl);
    let action = BatchOrder::new(vec![order]).with_grouping(Grouping::NormalTpsl);
    log_action("BatchOrder (stop loss)", &action);

    let result = send_action(action).await;
    log_response("BatchOrder (stop loss)", &result);
}

#[tokio::test]
async fn test_batch_order_trigger_take_profit() {
    let order = make_trigger_order(TEST_ASSET, false, "120.0", "0.01", "115.0", TpSl::Tp);
    let action = BatchOrder::new(vec![order]).with_grouping(Grouping::NormalTpsl);
    log_action("BatchOrder (take profit)", &action);

    let result = send_action(action).await;
    log_response("BatchOrder (take profit)", &result);
}

// ============================================================================
// BatchCancel
// ============================================================================

#[tokio::test]
async fn test_batch_cancel_single() {
    // Note: This will fail if the order doesn't exist
    let action = BatchCancel::single(TEST_ASSET, 12345);
    log_action("BatchCancel (single)", &action);

    let result = send_action(action).await;
    log_response("BatchCancel (single)", &result);
}

#[tokio::test]
async fn test_batch_cancel_multiple() {
    let cancels = vec![
        CancelWire {
            a: TEST_ASSET,
            o: 12345,
        },
        CancelWire {
            a: TEST_ASSET,
            o: 12346,
        },
    ];
    let action = BatchCancel::new(cancels);
    log_action("BatchCancel (multiple)", &action);

    let result = send_action(action).await;
    log_response("BatchCancel (multiple)", &result);
}

// ============================================================================
// CancelByCloid
// ============================================================================

#[tokio::test]
async fn test_cancel_by_cloid() {
    let cancels = vec![CancelByCloidWire {
        asset: TEST_ASSET,
        cloid: "test-cloid-12345".to_string(),
    }];
    let action = CancelByCloid::new(cancels);
    log_action("CancelByCloid", &action);

    let result = send_action(action).await;
    log_response("CancelByCloid", &result);
}

// ============================================================================
// BatchModify
// ============================================================================

#[tokio::test]
async fn test_batch_modify() {
    let new_order = make_limit_order(TEST_ASSET, true, "101.0", "0.02");
    let action = BatchModify::single(12345, new_order);
    log_action("BatchModify", &action);

    let result = send_action(action).await;
    log_response("BatchModify", &result);
}

#[tokio::test]
async fn test_batch_modify_multiple() {
    let modifies = vec![
        ModifyWire {
            oid: 12345,
            order: make_limit_order(TEST_ASSET, true, "101.0", "0.02"),
        },
        ModifyWire {
            oid: 12346,
            order: make_limit_order(TEST_ASSET, true, "102.0", "0.03"),
        },
    ];
    let action = BatchModify::new(modifies);
    log_action("BatchModify (multiple)", &action);

    let result = send_action(action).await;
    log_response("BatchModify (multiple)", &result);
}

// ============================================================================
// ScheduleCancel
// ============================================================================

#[tokio::test]
async fn test_schedule_cancel_at_time() {
    // Schedule cancel 5 minutes in the future (UTC milliseconds)
    let five_minutes_from_now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
        + 300_000;

    let action = ScheduleCancel::at(five_minutes_from_now);
    log_action("ScheduleCancel (at time)", &action);

    let result = send_action(action).await;
    log_response("ScheduleCancel (at time)", &result);
}

#[tokio::test]
async fn test_schedule_cancel_now() {
    let action = ScheduleCancel::now(); // Cancel all orders immediately
    log_action("ScheduleCancel (now)", &action);

    let result = send_action(action).await;
    log_response("ScheduleCancel (now)", &result);
}
