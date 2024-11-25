use std::sync::Arc;

use axum::{extract::State, Json};

use super::{ds::AppState, mode::ModeEnum};

pub async fn switch_to_auto(app_state: State<Arc<AppState>>) -> Json<&'static str> {
    let auto_mode = app_state.watering_system.auto_mode.read().await.clone();
    app_state
        .watering_system
        .switch_mode(ModeEnum::Auto(auto_mode))
        .await;
    Json("Switched to Auto Mode")
}

pub async fn switch_to_manual(app_state: State<Arc<AppState>>) -> Json<&'static str> {
    let manual_mode = app_state.watering_system.manual_mode.read().await.clone();
    app_state
        .watering_system
        .switch_mode(ModeEnum::Manual(manual_mode))
        .await;
    Json("Switched to Manual Mode")
}

pub async fn switch_to_wizard(app_state: State<Arc<AppState>>) -> Json<&'static str> {
    let wizard_mode = app_state.watering_system.wizard_mode.read().await.clone();
    app_state
        .watering_system
        .switch_mode(ModeEnum::Wizard(wizard_mode))
        .await;
    Json("Switched to Wizard Mode")
}