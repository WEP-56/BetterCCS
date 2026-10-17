use cc_switch_lib::AppError;

#[path = "support.rs"]
mod support;
use support::{create_test_state, ensure_test_home, reset_test_fs, test_mutex};

// 测试使用 Mutex 进行串行化，跨 await 持锁是预期行为
#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn pricing_model_source_round_trips() {
    let _guard = test_mutex().lock().unwrap_or_else(|e| e.into_inner());
    reset_test_fs();
    let _home = ensure_test_home();

    let state = create_test_state().expect("create test state");

    let default = state
        .db
        .get_pricing_model_source("claude")
        .await
        .expect("read default pricing model source");
    assert_eq!(default, "response");

    state
        .db
        .set_pricing_model_source("claude", "request")
        .await
        .expect("set pricing model source");
    let updated = state
        .db
        .get_pricing_model_source("claude")
        .await
        .expect("read updated pricing model source");
    assert_eq!(updated, "request");

    let err = state
        .db
        .set_pricing_model_source("claude", "invalid")
        .await
        .expect_err("invalid pricing model source should error");
    // 错误是 Localized 类型（支持 i18n）
    match err {
        AppError::Localized { key, .. } => {
            assert_eq!(key, "error.invalidPricingMode");
        }
        other => panic!("expected localized error, got {other:?}"),
    }
}
