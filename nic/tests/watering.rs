use chrono::{TimeZone, Utc};
use nic::{
    test::utils::{mock_db::*, set_app_and_ws, set_ws0},
    utils::{ux_ts_to_string, load_sectors_into_hashmap, parse_datetime_to_utc_timestamp, sod, start_log},
    watering::{
        ds::{DailyPlan, SectorInfo, WaterSector},
        modes::Mode,
        state_machine::SMState,
        water_window::WaterWin,
        watering_system::run_watering_system,
    },
};
use std::sync::Arc;

#[test]
fn watering_at_right_times() {
    let now = parse_datetime_to_utc_timestamp("2024-11-29T17:00:00+00:00", "%Y-%m-%dT%H:%M:%S%z").unwrap();
    let allowed_timeframe = WaterWin::new(now, 22, 8);
    let mock_db = Some(Arc::new(MockDatabase::new()));
    let mut ws = set_ws0(now, Some(Mode::Wizard), mock_db).unwrap();
    let time_provider = ws.time_provider.clone();

    // Set up WizardMode with sectors and schedule
    ws.sm.timeframe = allowed_timeframe;

    let sectors = load_sectors_into_hashmap(vec![
        SectorInfo::build(1, 2.5, 2.5, 30 * 60, 0., 5., 0),
        SectorInfo::build(2, 2.5, 2.5, 30 * 60, 0., 4., 0),
    ]);
    ws.sm.sectors = sectors;

    let daily_plan = DailyPlan(vec![
        WaterSector::new(1, sod(now) + (22 * 3600), 30 * 60), // Sector 1, start at 22:00 UTC, 30 mins duration
    ]);
    ws.sm.mode_wizard.daily_plan = vec![daily_plan];

    // Simulate watering execution at various times
    let test_cases: Vec<(i64, bool)> = vec![
        (parse_datetime_to_utc_timestamp("2024-11-29T17:00:00+00:00", "%Y-%m-%dT%H:%M:%S%z").unwrap(), false), // Before allowed timeframe
        (parse_datetime_to_utc_timestamp("2024-11-29T22:00:00+00:00", "%Y-%m-%dT%H:%M:%S%z").unwrap(), true), // Within allowed timeframe
        (parse_datetime_to_utc_timestamp("2024-11-30T07:00:00+00:00", "%Y-%m-%dT%H:%M:%S%z").unwrap(), false), // After allowed timeframe
    ];

    for (time, should_water) in test_cases {
        println!("Testing for time: {:?}", ux_ts_to_string(time));
        time_provider.set(time);

        // Call the execute function
        ws.sm.update(time_provider.now());

        {
            // Verify watering state
            if should_water {
                assert_ne!(ws.sm.state, SMState::Idle, "Expected watering to start.");
                ws.sm.stop();
            } else {
                assert_eq!(ws.sm.state, SMState::Idle, "Expected no watering outside timeframe.");
            }

            // Reset state
            ws.sm.state = SMState::Idle;
        }
    }
}

#[tokio::test]
async fn run_watering_system_fast_forward() {
    let now = Utc.with_ymd_and_hms(2024, 12, 1, 22, 0, 0).unwrap().timestamp(); // 6:00 AM UTC
    let (app_state, mut ws) = set_app_and_ws(now, Some(Mode::Wizard)).unwrap();
    let time_provider = ws.time_provider.clone();
    let allowed_timeframe = WaterWin::new(now, 22, 8); // 10 PM to 6 AM
    ws.sm.timeframe = allowed_timeframe;
    start_log(Some(time_provider.clone()));

    // Simulation parameters
    let simulation_duration_seconds = 13 * 24 * 3600;

    let sectors = load_sectors_into_hashmap(vec![
        SectorInfo::build(1, 2.5, 1.6, 30 * 60, 0., 0.29, 0),
        SectorInfo::build(2, 2.5, 1.6, 30 * 60, 0., 0.29, 0),
    ]);
    ws.sm.sectors = sectors;

    let auto_daily_plan = DailyPlan(
        vec![WaterSector::new(1, sod(now) + (8 * 3600), 60 * 60)], // Sector 1: 8 AM start
    );

    ws.sm.mode_auto.daily_plan = vec![auto_daily_plan];

    // TODO: now it is in Wizard Mode.  Make a variant of the test to switch modes
    // let mut active_mode = app_state.watering_system.active_mode.write().await;
    // let auto_mode = app_state.watering_system.auto_mode.read().await.clone();
    // *active_mode = ModeEnum::Auto(auto_mode);
    assert_eq!(ws.sm.current_mode, Mode::Wizard, "Active mode should be Wizard Mode.");

    let (_shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    // let rx_clone = shutdown_rx.clone();

    // Broadcast channel for control signals
    _ = run_watering_system(
        app_state.clone(),
        Some(Mode::Wizard),
        shutdown_rx,
        Some(now + simulation_duration_seconds),
        Some(&mut ws),
    )
    .await;

    // Validate the results
    for sector in ws.sm.sectors.values() {
        assert!(sector.progress > 0.0, "Sector {} should have positive progress after simulation.", sector.id);
    }
}
