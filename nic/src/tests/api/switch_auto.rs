use axum::{body::Body, extract::Request, routing::post, Router};
use hyper::StatusCode;
use tower::ServiceExt;

use crate::{
    db::mock::MockDatabase,
    watering::{api::switch_to_auto, ds::AppState, mode::ModeEnum},
};

#[tokio::test]
async fn test_switch_to_auto() {
    let db = MockDatabase::new();
    let app_state = AppState::new_with_mock(db).await;

    let app = Router::new()
        .route("/switch/auto", post(switch_to_auto))
        .with_state(app_state.clone());

    let request = Request::builder()
        .method("POST")
        .uri("/switch/auto")
        .header("Content-Type", "application/json")
        .extension(app_state.clone())
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    assert!(matches!(
        *app_state.watering_system.active_mode.read().await,
        ModeEnum::Auto(_)
    ));
}
