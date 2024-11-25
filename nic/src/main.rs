use axum::routing::post;
use axum::{routing::get, Router};
use axum_server::Server;
use db::Database;
use watering::api::{switch_to_auto, switch_to_manual, switch_to_wizard};
use watering::state_machine::run_watering_system;
use std::{error::Error, sync::Arc};
use tokio::sync::broadcast;
use tokio::sync::Mutex;
use watering::ds::AppState;
use watering::ds::ControlSignal;
use watering::state_machine::WateringSystem;

mod db;
mod watering;
mod weather;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let db = Database::new("watering_system.db")?;

    // Broadcast channel for real-time updates
    let (tx, rx) = broadcast::channel::<ControlSignal>(100);
    let tx = Arc::new(tx);
    let rx = Arc::new(Mutex::new(rx)); // Wrap in Mutex for safe access across tasks

    // Initialize watering state machine and modes
    let watering_system = WateringSystem::new().await;
    let app_state = Arc::new(AppState {
        db: db.clone(),
        watering_system: Arc::new(watering_system),
    });

    // Start monitoring tasks
    tokio::spawn(weather::mqtt_mon::monitor_mqtt(tx.clone()));
    tokio::spawn(weather::mqtt_mon::monitor_udp(tx.clone(), db.clone()));

    let app = Router::new()
        .route("/devices", get(weather::api::list_devices))
        .route("/weather", get(weather::api::query_weather))
        .route("/state", get(watering::query_state))
        .route("/switch/auto", post(switch_to_auto))
        .route("/switch/manual", post(switch_to_manual))
        .route("/switch/wizard", post(switch_to_wizard))
        .route("/command", get(watering::send_command)) // Example: command=stop or command=auto
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
