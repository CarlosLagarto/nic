use crate::{
    db::DatabaseTrait,
    sensors::interface::SensorController,
    watering::{
        ds::{AppState, EventType, WateringState},
        mode::ModeEnum,
    },
};
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub async fn switch_to_auto<C: SensorController + 'static, D: DatabaseTrait + 'static>(
    app_state: State<Arc<AppState<C, D>>>,
) -> Json<&'static str> {
    let auto_mode = app_state.watering_system.auto_mode.read().await.clone();
    app_state.watering_system.switch_mode(ModeEnum::Auto(auto_mode)).await;
    Json("Switched to Auto Mode")
}

pub async fn switch_to_manual<C: SensorController + 'static, D: DatabaseTrait + 'static>(
    app_state: State<Arc<AppState<C, D>>>,
) -> Json<&'static str> {
    let manual_mode = app_state.watering_system.manual_mode.read().await.clone();
    app_state.watering_system.switch_mode(ModeEnum::Manual(manual_mode)).await;
    Json("Switched to Manual Mode")
}

pub async fn switch_to_wizard<C: SensorController + 'static, D: DatabaseTrait + 'static>(
    app_state: State<Arc<AppState<C, D>>>,
) -> Json<&'static str> {
    let wizard_mode = app_state.watering_system.wizard_mode.read().await.clone();
    app_state.watering_system.switch_mode(ModeEnum::Wizard(wizard_mode)).await;
    Json("Switched to Wizard Mode")
}

#[derive(Serialize)]
pub struct WateringStateResponse {
    pub mode: String,
    pub state: String,
    pub current_cycle: Option<String>,
}

pub async fn get_state<C: SensorController, D: DatabaseTrait>(
    State(app_state): State<Arc<AppState<C, D>>>,
) -> Json<WateringStateResponse> {
    let active_mode = app_state.watering_system.active_mode.read().await;
    let mode = match &*active_mode {
        ModeEnum::Auto(_) => EventType::Auto,
        ModeEnum::Manual(_) => EventType::Manual,
        ModeEnum::Wizard(_) => EventType::Wizard,
    };

    let state_machine = app_state.watering_system.state_machine.read().await;
    let state = match state_machine.state {
        WateringState::Idle => "Idle".to_string(),
        WateringState::Activating(sector) => format!("Activating sector {}", sector),
        WateringState::Watering(sector, duration) => {
            format!("Watering sector {} for {:.2} minutes", sector, (duration as f64 / 60.))
        }
        WateringState::Deactivating(sector) => format!("Deactivating sector {}", sector),
    };

    let current_cycle = state_machine
        .cycle
        .as_ref()
        .map(|cycle| format!("Cycle ID: {}, Instructions: {:?}", cycle.id, cycle.instructions));

    Json(WateringStateResponse { mode: mode.to_string(), state, current_cycle })
}

pub async fn send_command<C: SensorController, D: DatabaseTrait>(
    State(_app_state): State<Arc<AppState<C, D>>>,
) -> String {
    // Parse command and modify system state
    // TODO:
    "Command received".to_string()
}

#[derive(Serialize, Deserialize)]
pub struct CycleResponse {
    pub id: Option<u32>,
    pub instructions: Option<Vec<(u32, String)>>, // Instruction details: sector and duration
}

pub async fn get_cycle<C: SensorController, D: DatabaseTrait>(
    State(app_state): State<Arc<AppState<C, D>>>,
) -> Json<CycleResponse> {
    let state_machine = app_state.watering_system.state_machine.read().await;

    if let Some(cycle) = &state_machine.cycle {
        let instructions = cycle
            .instructions
            .iter()
            .map(|(sector_id, duration)| (*sector_id, format!("{} minutes", duration)))
            .collect();

        Json(CycleResponse { id: Some(cycle.id), instructions: Some(instructions) })
    } else {
        Json(CycleResponse { id: None, instructions: None })
    }
}
