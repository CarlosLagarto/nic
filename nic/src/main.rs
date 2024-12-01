use axum::routing::post;
use axum::{routing::get, Router};
use axum_server::Server;
use nic::api::{get_cycle, get_state, send_command, switch_to_auto, switch_to_manual, switch_to_wizard};
use nic::db::Database;
use nic::sensors::interface::RealSensorController;
use nic::utils::start_log;
use nic::watering::ds::{AppState, ControlSignal};
use nic::watering::watering_system::run_watering_system;
use nic::weather;
use std::{error::Error, sync::Arc};
use tokio::sync::{broadcast, Mutex};
use tracing::{debug, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    start_log();

    info!("Starting application...");
    debug!("test");

    let db = Arc::new(Database::new("watering_system.db")?);

    // Broadcast channel for real-time updates
    let (tx, rx) = broadcast::channel::<ControlSignal>(100);
    let tx = Arc::new(tx);
    let rx = Arc::new(Mutex::new(rx));

    let controller = Arc::new(RealSensorController {});
    let app_state = AppState::new(db.clone(), controller).await?;

    tokio::spawn(weather::mqtt_mon::monitor_mqtt(tx.clone()));
    tokio::spawn(weather::mqtt_mon::monitor_udp(tx.clone(), db.clone()));

    let app = Router::new()
        .route("/devices", get(weather::api::list_devices))
        .route("/weather", get(weather::api::query_weather))
        .route("/state", get(get_state))
        .route("/cycle", get(get_cycle))
        .route("/switch/auto", post(switch_to_auto))
        .route("/switch/manual", post(switch_to_manual))
        .route("/switch/wizard", post(switch_to_wizard))
        .route("/command", get(send_command)) // Example: command=stop or command=auto
        .with_state(app_state.clone());

    // Start watering system loop
    tokio::spawn(async move {
        run_watering_system(app_state.clone(), rx).await;
    });

    info!("Starting HTTP server on http://0.0.0.0:8080");
    Server::bind("0.0.0.0:8080".parse().unwrap()).serve(app.into_make_service()).await.unwrap();
    Ok(())
}
