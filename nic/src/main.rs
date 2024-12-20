use nic::api::run_web_server;
use nic::config::run_options::get_args;
use nic::config::Config;
use nic::db::Database;
use nic::sensors::interface::RealSensorController;
use nic::time::RealTimeProvider;
use nic::utils::{init_broadcast_channels, init_channels, start_log};
use nic::watering::ds::AppState;
use nic::watering::modes::Mode;
use nic::watering::watering_system::run_watering_system;
use nic::weather;
use std::{error::Error, sync::Arc};
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = get_args();
    let cfg = if let Some(cfg_str) = args.cfg_str { Config::load_from_str(&cfg_str) } else { Config::load(args) };
    start_log(None);

    info!("Starting application...");

    let db = Arc::new(Database::new(&cfg.database.name)?);

    let (sm_tx, sm_rx) = init_channels();
    let (web_tx, web_rx) = init_broadcast_channels();

    let (_shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

    let controller = Arc::new(RealSensorController {});
    let time_provider = Arc::new(RealTimeProvider);
    // TODO: read from config and db, in case is not a fresh start
    let app_state = AppState::new(db.clone(), controller, time_provider, sm_tx.clone(), sm_rx, web_tx, web_rx).await?;

    tokio::spawn(weather::mqtt_mon::monitor_mqtt(sm_tx.clone()));
    tokio::spawn(weather::mqtt_mon::monitor_udp(sm_tx.clone(), db.clone()));

    // Start watering system loop
    let app_state_clone = app_state.clone();
    let rx_clone = shutdown_rx.clone();
    tokio::spawn(async move {
        run_watering_system(app_state_clone, Some(Mode::Auto), rx_clone, None, None, cfg.watering)
            .await
            .unwrap_or_else(|e| error!("HTTP server error: {}", e)); // TODO
    });

    let app_state_clone = app_state.clone();
    tokio::spawn(async move {
        let ip_addr = cfg.web_server.address.parse().unwrap();
        if let Err(e) = run_web_server(app_state_clone, ip_addr, shutdown_rx).await {
            error!("Web server error: {}", e);
        }
    })
    .await?;

    Ok(())
}
