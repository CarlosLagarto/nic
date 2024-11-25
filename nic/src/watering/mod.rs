pub mod api;
pub mod ds;
pub mod interface;
pub mod mode;
pub mod mode_auto;
pub mod mode_manual;
pub mod mode_wizard;
pub mod schedule;
pub mod state_machine;

use axum::extract::State;
use ds::AppState;
use std::sync::Arc;

pub async fn query_state(State(_app_state): State<Arc<AppState>>) -> String {
    // Return current watering system state
    "Current state information".to_string()
}

pub async fn send_command(State(_app_state): State<Arc<AppState>>) -> String {
    // Parse command and modify system state
    "Command received".to_string()
}
