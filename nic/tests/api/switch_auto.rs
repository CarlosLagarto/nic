use axum::{body::Body, extract::Request, routing::post, Router};
use hyper::StatusCode;
use nic::{
    api::{get_cycle, get_state, switch_to_auto, switch_to_manual, switch_to_wizard, CycleResponse},
    watering::{ds::Cycle, mode::ModeEnum},
};
use std::usize;
use test_utilities::common::{
    mock_db::{new_with_mock, MockDatabase},
    mock_sensors::set_sensor_controller,
};
use tower::ServiceExt;

#[tokio::test]
async fn test_switch_to_auto() {
    let app_state = set_app_state().await;

    let app = Router::new().route("/switch/auto", post(switch_to_auto)).with_state(app_state.clone());

    let request = Request::builder()
        .method("POST")
        .uri("/switch/auto")
        .header("Content-Type", "application/json")
        .extension(app_state.clone())
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    assert!(matches!(*app_state.watering_system.active_mode.read().await, ModeEnum::Auto(_)));
}

#[tokio::test]
async fn test_switch_to_manual() {
    let app_state = set_app_state().await;

    let app = Router::new().route("/switch/manual", post(switch_to_manual)).with_state(app_state.clone());

    let request = Request::builder()
        .method("POST")
        .uri("/switch/manual")
        .header("Content-Type", "application/json")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    assert!(matches!(*app_state.watering_system.active_mode.read().await, ModeEnum::Manual(_)));
}

#[tokio::test]
async fn test_switch_to_wizard() {
    let app_state = set_app_state().await;

    let app = Router::new().route("/switch/wizard", post(switch_to_wizard)).with_state(app_state.clone());

    let request = Request::builder()
        .method("POST")
        .uri("/switch/wizard")
        .header("Content-Type", "application/json")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    assert!(matches!(*app_state.watering_system.active_mode.read().await, ModeEnum::Wizard(_)));
}

#[tokio::test]
async fn test_get_state() {
    let app_state = set_app_state().await;

    let app = Router::new().route("/state", post(get_state)).with_state(app_state.clone());

    let request = Request::builder()
        .method("POST")
        .uri("/state")
        .header("Content-Type", "application/json")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let state: String = String::from_utf8(body.to_vec()).unwrap();
    assert!(state.contains("Idle") || state.contains("Activating") || state.contains("Watering"));
}

#[tokio::test]
async fn test_get_cycle() {
    let app_state = set_app_state().await;

    {
        let mut state_machine = app_state.watering_system.state_machine.write().await;
        state_machine.start_cycle(Cycle {
            id: 1,
            instructions: vec![(1, chrono::Duration::minutes(15)), (2, chrono::Duration::minutes(20))],
        });
    }

    let app = Router::new().route("/cycle", post(get_cycle)).with_state(app_state.clone());

    let request = Request::builder()
        .method("POST")
        .uri("/cycle")
        .header("Content-Type", "application/json")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let cycle: CycleResponse = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(cycle.id, Some(1));
    assert_eq!(cycle.instructions.as_ref().unwrap().len(), 2);
    assert!(cycle.instructions.as_ref().unwrap().contains(&(1, "15 minutes".to_string())));
}
