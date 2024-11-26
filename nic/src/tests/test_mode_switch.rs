use std::sync::Arc;

use crate::{
    db::mock::MockDatabase,
    watering::{ds::AppState, mode::ModeEnum},
};

#[tokio::test]
async fn test_mode_switching() {
    let db = MockDatabase::new();
    let app_state = Arc::new(AppState::new_with_mock(db).await);

    assert!(matches!(
        *app_state.watering_system.active_mode.read().await,
        ModeEnum::Auto(_)
    ));

    let manual_mode = app_state.watering_system.manual_mode.read().await.clone();
    app_state
        .watering_system
        .switch_mode(ModeEnum::Manual(manual_mode))
        .await;

    assert!(matches!(
        *app_state.watering_system.active_mode.read().await,
        ModeEnum::Manual(_)
    ));
}
