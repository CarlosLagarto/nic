use chrono::{TimeZone, Utc};
use hyper::StatusCode;
use nic::api::run_web_server;
use nic::test::utils::mock_cfg::mock_cfg;
use nic::test::utils::mock_db::mock_sector;
use nic::test::utils::set_app_and_ws0;
use nic::utils::{load_sectors_into_hashmap, start_log};
use nic::watering::ds::{DailyPlan, WaterSector};
use nic::watering::modes::*;
use nic::watering::watering_system::run_watering_system;
use nic::{
    api::{CycleResponse, WateringStateResponse},
    watering::ds::CtrlSignal,
};
use tracing::error;

fn mock_schedule(current_time: i64) -> Vec<DailyPlan> {
    vec![DailyPlan(vec![
        WaterSector::new(1, current_time + 300, 900), // Start 5 min later, lasts 15 min
        WaterSector::new(2, current_time + 1500, 1200), // Start 25 min later, lasts 20 min
    ])]
}

#[tokio::test]
async fn watering_system_response_to_routes_function_calls() {
    let current_time = Utc.with_ymd_and_hms(2023, 11, 25, 22, 0, 0).unwrap().timestamp();
    let cfg = mock_cfg();
    let (app_state, mut ws) = set_app_and_ws0(current_time, Some(Mode::Auto), cfg.watering).unwrap();
    let app_state_clone = app_state.clone();
    ws.sm.sectors = load_sectors_into_hashmap(mock_sector());
    ws.sm.mode_auto = ModeAuto { daily_plan: mock_schedule(current_time) };
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    let watering_system_task = tokio::spawn(async move {
        let _ = run_watering_system(app_state_clone, Some(Mode::Auto), shutdown_rx, None, Some(&mut ws), cfg.watering)
            .await;
    });

    app_state.sm_tx.send(CtrlSignal::ChgMode(Mode::Manual)).unwrap();
    app_state.sm_tx.send(CtrlSignal::GetState).unwrap();
    if let Ok(CtrlSignal::GetStateResponse(resp)) = app_state.sm_rx.lock().await.try_recv() {
        assert_eq!(resp.mode.as_ref().unwrap(), "manual");
        assert!(resp.mode.is_some());
        assert!(resp.state.is_some());
    }

    app_state.sm_tx.send(CtrlSignal::ChgMode(Mode::Auto)).unwrap();
    app_state.sm_tx.send(CtrlSignal::GetState).unwrap();
    if let Ok(CtrlSignal::GetStateResponse(resp)) = app_state.sm_rx.lock().await.try_recv() {
        assert_eq!(resp.mode.as_ref().unwrap(), "auto");
        assert!(resp.mode.is_some());
        assert!(resp.state.is_some());
    }

    app_state.sm_tx.send(CtrlSignal::GetCycle).unwrap();
    if let Ok(CtrlSignal::GetCycleResponse(resp)) = app_state.sm_rx.lock().await.try_recv() {
        assert!(resp.error.is_none());
        println!("{:?}", resp);
    }

    app_state.sm_tx.send(CtrlSignal::StopMachine).unwrap();
    app_state.sm_tx.send(CtrlSignal::GetState).unwrap();
    if let Ok(CtrlSignal::GetStateResponse(resp)) = app_state.sm_rx.lock().await.try_recv() {
        assert_eq!(resp.mode.as_ref().unwrap(), "manual");
        assert!(resp.mode.is_some());
        assert!(resp.state.is_some());
    }

    // Clean up
    _ = shutdown_tx.send(true);
    watering_system_task.abort();
}

#[tokio::test]
async fn test_full_web_server() {
    let current_time = Utc.with_ymd_and_hms(2023, 11, 25, 22, 0, 0).unwrap().timestamp();
    let cfg = mock_cfg();
    let (app_state, mut ws) = set_app_and_ws0(current_time, Some(Mode::Auto), cfg.watering).unwrap();
    let app_state_clone = app_state.clone();
    ws.sm.sectors = load_sectors_into_hashmap(mock_sector());
    ws.sm.mode_auto = ModeAuto { daily_plan: mock_schedule(current_time) };

    let time_provider = ws.time_provider.clone();
    start_log(Some(time_provider.clone()));

    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    let rx_clone = shutdown_rx.clone();
    let watering_system_task = tokio::spawn(async move {
        let _ =
            run_watering_system(app_state_clone, Some(Mode::Auto), rx_clone, None, Some(&mut ws), cfg.watering).await;
    });

    let app_state_clone = app_state.clone();
    let rx_clone = shutdown_rx.clone();
    let str_ip_addr = "127.0.0.1:3010";
    let ip_addr = str_ip_addr.parse().unwrap();
    let server_task = tokio::spawn(async move {
        if let Err(e) = run_web_server(app_state_clone, ip_addr, rx_clone).await {
            error!(error=?e, "Web server error.");
        }
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let client = reqwest::Client::new();

    // Test `/switch/auto` route
    let response = client.post(format!("http://{}/switch/auto", str_ip_addr)).send().await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Test `/state` route
    let response = client.get(format!("http://{}/state", str_ip_addr)).send().await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let state_response: WateringStateResponse = response.json().await.unwrap();
    assert!(state_response.mode.is_some());
    assert!(state_response.state.is_some());

    // Test `/cycle` route
    let response = client.get(format!("http://{}/cycle", str_ip_addr)).send().await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let cycle_response: CycleResponse = response.json().await.unwrap();
    assert!(cycle_response.error.is_none());
    assert!(cycle_response.id.is_none());
    assert!(cycle_response.instructions.is_none());

    // Test `/command` route
    let response = client.get(format!("http://{}/command?command=stop", str_ip_addr)).send().await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Clean up
    _ = shutdown_tx.send(true);
    server_task.abort();
    watering_system_task.abort();
}
