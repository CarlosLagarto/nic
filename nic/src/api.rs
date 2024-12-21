use crate::{
    watering::{
        ds::{AppState, CtrlSignal},
        modes::Mode,
    },
    weather::api::{list_devices, query_weather},
};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::Path;
use axum::routing::post;
use axum::{extract::State, Json};
use axum::{routing::get, Router};
use serde::{Deserialize, Serialize};
use std::{error::Error, net::SocketAddr};
use std::{str::FromStr, sync::Arc};
use tokio::{signal, sync::watch};
use tracing::info;

pub async fn run_web_server(
    app_state: Arc<AppState>, ip_addr: SocketAddr, stop_signal: watch::Receiver<bool>,
) -> Result<(), Box<dyn Error>> {
    let app = Router::new()
        .route("/ws/weather", get(ws_handler))
        .route("/devices", get(list_devices))
        .route("/weather", get(query_weather))
        .route("/state", get(get_state))
        .route("/cycle", get(get_cycle))
        .route("/switch/:mode", post(switch_mode))
        .route("/command", get(send_command)) // Example: command=stop or command=auto
        .with_state(app_state);

    info!("Starting HTTP server on http://{}", ip_addr);
    let listener = tokio::net::TcpListener::bind(ip_addr).await.unwrap();
    axum::serve(listener, app).with_graceful_shutdown(shutdown_signal(stop_signal)).await?;
    Ok(())
}

// Handler for the WebSocket upgrade
async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> impl axum::response::IntoResponse {
    ws.on_upgrade(move |socket| handle_ws_connection(socket, state))
}

// Handle the WebSocket connection
async fn handle_ws_connection(mut socket: WebSocket, state: Arc<AppState>) {
    let mut web_rx = state.web_rx.resubscribe();

    // Send updates to the client
    while let Ok(update) = web_rx.recv().await {
        if let CtrlSignal::WeatherData(data) = update {
            if socket.send(Message::Text(serde_json::to_string(&data).unwrap())).await.is_err() {
                break; // Exit loop if client disconnects
            }
        }
    }
}

pub async fn switch_mode(Path(mode): Path<String>, app_state: State<Arc<AppState>>) -> Json<String> {
    match Mode::from_str(&mode) {
        Ok(valid_mode) => {
            app_state.sm_tx.send(CtrlSignal::ChgMode(valid_mode)).unwrap();
            Json(format!("Switched to {} mode", valid_mode))
        }
        Err(_) => Json("error: Invalid mode".to_owned()),
    }
}

async fn shutdown_signal(stop_signal: watch::Receiver<bool>) {
    let ctrl_c = async {
        signal::ctrl_c().await.expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    let stop_signal_task = async {
        while !*stop_signal.borrow() {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
        _ = stop_signal_task=>{}
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
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

pub async fn get_state(State(app_state): State<Arc<AppState>>) -> Json<WateringStateResponse> {
    let mut web_rx = app_state.web_rx.resubscribe();
    _ = app_state.sm_tx.send(CtrlSignal::GetState); // TODO
    loop {
        match web_rx.recv().await {
            Ok(resp) => {
                if let CtrlSignal::GetStateResponse(resp) = resp {
                    return Json(resp);
                }
            }
            Err(_e) => return Json(WateringStateResponse::new_error()), // TODO , return error messae
        }
    }
}

pub async fn send_command(State(_app_state): State<Arc<AppState>>) -> String {
    // Parse command and modify system state
    // TODO:
    "Command received".to_string()
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CycleResponse {
    pub error: Option<String>,
    pub id: Option<i64>,
    pub instructions: Option<Vec<(u32, String)>>, // Instruction details: sector and duration
}

impl CycleResponse {
    pub fn new_error() -> Self {
        Self { error: Some("Error".to_owned()), id: None, instructions: None }
    }
}
pub async fn get_cycle(State(app_state): State<Arc<AppState>>) -> Json<CycleResponse> {
    let mut web_rx = app_state.web_rx.resubscribe();
    _ = app_state.sm_tx.send(CtrlSignal::GetCycle); //TODO
    loop {
        match web_rx.recv().await {
            Ok(resp) => {
                if let CtrlSignal::GetCycleResponse(resp) = resp {
                    return Json(resp);
                }
            }
            Err(_e) => return Json(CycleResponse::new_error()), // TODO , return error messae
        }
    }
}
