use crate::{
    db::DatabaseTrait,
    sensors::interface::SensorController,
    time::TimeProvider,
    watering::ds::{AppState, ControlSignal},
};
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub async fn switch_to_auto<C, D, T>(app_state: State<Arc<AppState<C, D, T>>>) -> Json<&'static str>
where
    C: SensorController + 'static,
    D: DatabaseTrait + 'static,
    T: TimeProvider + 'static,
{
    _ = app_state.tx.send(ControlSignal::SwitchToAuto);
    Json("Switched to Auto Mode")
}

pub async fn switch_to_manual<C, D, T>(app_state: State<Arc<AppState<C, D, T>>>) -> Json<&'static str>
where
    C: SensorController + 'static,
    D: DatabaseTrait + 'static,
    T: TimeProvider + 'static,
{
    _ = app_state.tx.send(ControlSignal::SwitchToManual); // TODO
    Json("Switched to Manual Mode")
}

pub async fn switch_to_wizard<C, D, T>(app_state: State<Arc<AppState<C, D, T>>>) -> Json<&'static str>
where
    C: SensorController + 'static,
    D: DatabaseTrait + 'static,
    T: TimeProvider + 'static,
{
    _ = app_state.tx.send(ControlSignal::SwitchToWizard); // TODO
    Json("Switched to Wizard Mode")
}

#[derive(Serialize, Debug, Clone)]
pub struct WateringStateResponse {
    pub error: Option<String>,
    pub mode: Option<String>,
    pub state: Option<String>,
    pub current_cycle: Option<String>,
}

impl WateringStateResponse {
    pub fn new_error() -> Self {
        Self { error: Some("Error".to_owned()), mode: None, state: None, current_cycle: None }
    }
}

pub async fn get_state<C, D, T>(State(app_state): State<Arc<AppState<C, D, T>>>) -> Json<WateringStateResponse>
where
    C: SensorController,
    D: DatabaseTrait,
    T: TimeProvider,
{
    _ = app_state.tx.send(ControlSignal::GetState); // TODO
    loop {
        match app_state.rx.lock().await.recv().await {
            Ok(resp) => {
                if let ControlSignal::GetStateResponse(resp) = resp{
                    return Json(resp);
                    // break;
                }
            }
            Err(_e) => return Json(WateringStateResponse::new_error()), // TODO , return error messae
        }
    }
}

pub async fn send_command<C, D, T>(State(_app_state): State<Arc<AppState<C, D, T>>>) -> String
where
    C: SensorController,
    D: DatabaseTrait,
    T: TimeProvider,
{
    // Parse command and modify system state
    // TODO:
    "Command received".to_string()
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CycleResponse {
    pub error: Option<String>,
    pub id: Option<u32>,
    pub instructions: Option<Vec<(u32, String)>>, // Instruction details: sector and duration
}

impl CycleResponse {
    pub fn new_error() -> Self {
        Self { error: Some("Error".to_owned()), id: None, instructions: None }
    }
}
pub async fn get_cycle<C, D, T>(State(app_state): State<Arc<AppState<C, D, T>>>) -> Json<CycleResponse>
where
    C: SensorController,
    D: DatabaseTrait,
    T: TimeProvider,
{
    _ = app_state.tx.send(ControlSignal::GetCycle); //TODO
    loop {
        match app_state.rx.lock().await.recv().await {
            Ok(resp) => {
                if let ControlSignal::GetCycleResponse(resp) = resp {
                    return Json(resp);
                    // break;
                }
            }
            Err(_e) => return Json(CycleResponse::new_error()), // TODO , return error messae
        }
    }
}
