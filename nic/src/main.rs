use axum::routing::post;
use axum::{routing::get, Router};
use axum_server::Server;
use nic::api::{
    get_cycle, get_state, send_command, switch_to_auto, switch_to_manual, switch_to_wizard,
};
use nic::db::Database;
use nic::watering::ds::AppState;
use nic::watering::ds::ControlSignal;
use nic::watering::interface::RealSensorController;
use nic::watering::state_machine::run_watering_system;
use nic::weather;
use std::{error::Error, sync::Arc};
use tokio::sync::broadcast;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let db = Database::new("watering_system.db")?;

    // Broadcast channel for real-time updates
    let (tx, rx) = broadcast::channel::<ControlSignal>(100);
    let tx = Arc::new(tx);
    let rx = Arc::new(Mutex::new(rx));
    let controller = Arc::new(RealSensorController {});
    let app_state = AppState::new(db.clone(), controller).await;

    tokio::spawn(weather::mqtt_mon::monitor_mqtt(tx.clone()));
    tokio::spawn(weather::mqtt_mon::monitor_udp(tx.clone(), db.clone()));

    let app = Router::new()
        .route("/devices", get(weather::api::list_devices))
        .route("/weather", get(weather::api::query_weather))
        .route("/state", get(get_state))
        .route("/state", get(get_cycle))
        .route("/switch/auto", post(switch_to_auto))
        .route("/switch/manual", post(switch_to_manual))
        .route("/switch/wizard", post(switch_to_wizard))
        .route("/command", get(send_command)) // Example: command=stop or command=auto
        .with_state(app_state.clone());

    // Start watering system loop
    tokio::spawn(async move {
        run_watering_system(app_state.clone(), rx).await;
    });

    println!("Starting HTTP server on http://0.0.0.0:8080");
    Server::bind("0.0.0.0:8080".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
    Ok(())
    // let sectors = db.load_sectors()?;
    // let cycles = db.load_cycles()?;

    // println!("Loaded sectors: {:?}", sectors);
    // println!("Loaded cycles: {:?}", cycles);

    // let auto_cycle = cycles.iter().find(|c| c.id == 1).cloned();
    // let manual_cycle = cycles.iter().find(|c| c.id == 2).cloned();

    // if auto_cycle.is_none() || manual_cycle.is_none() {
    //     eprintln!("Both Auto and Manual cycles must be defined in the database.");
    //     return Ok(());
    // }
}
