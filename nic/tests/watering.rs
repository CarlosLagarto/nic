use chrono::Datelike;
use nic::{
    test::utils::{mock_db::*, set_app_and_ws, set_ws0},
    time::TimeProvider,
    utils::{display_from_ts, load_sectors_into_hashmap, parse_datetime_to_utc_timestamp, sod, start_log},
    watering::{
        ds::{Cycle, EnvironmentalSignal, SectorInfo, WaterSector, WateringState},
        modes::ModeIdx,
        schedule::*,
        watering_system::run_watering_system,
    },
};
use std::sync::Arc;

#[tokio::test]
async fn watering_at_right_times() {
    let allowed_timeframe = AllowedTimeframe::new(22, 8);
    let now = parse_datetime_to_utc_timestamp("2024-11-29T17:00:00+00:00", "%Y-%m-%dT%H:%M:%S%z").unwrap();
    let mock_db = Some(Arc::new(MockDatabase::new()));
    let mut ws = set_ws0(now, Some(ModeIdx::Wizard), mock_db).unwrap();
    let time_provider = ws.time_provider.clone();

    // Set up WizardMode with sectors and schedule
    ws.water_state.timeframe = allowed_timeframe;

    let sectors = load_sectors_into_hashmap(vec![
        SectorInfo::build(1, 2.5, 1.5, 30 * 60, 0., 5.),
        SectorInfo::build(2, 12., 2., 40 * 60, 0., 4.),
    ]);
    ws.water_state.sectors = sectors;

    let schedule_entries = vec![ScheduleEntry {
        schedule_type: ScheduleType::Date(sod(now)), // Start of day for today
        start_times: vec![
            WaterSector::new(1, sod(now) + (22 * 3600), 30 * 60), // Sector 1, start at 22:00 UTC, 30 mins duration
        ],
    }];
    ws.water_state.mode_wizard.schedule = Schedule::new(schedule_entries);

    // Simulate watering execution at various times
    let test_cases: Vec<(i64, bool)> = vec![
        (parse_datetime_to_utc_timestamp("2024-11-29T17:00:00+00:00", "%Y-%m-%dT%H:%M:%S%z").unwrap(), false), // Before allowed timeframe
        (parse_datetime_to_utc_timestamp("2024-11-29T22:00:00+00:00", "%Y-%m-%dT%H:%M:%S%z").unwrap(), true), // Within allowed timeframe
        (parse_datetime_to_utc_timestamp("2024-11-30T07:00:00+00:00", "%Y-%m-%dT%H:%M:%S%z").unwrap(), false), // After allowed timeframe
    ];

    for (time, should_water) in test_cases {
        println!("Testing for time: {:?}", display_from_ts(time));
        time_provider.set(time);

        ws.water_state.start_cycle(Cycle { id: 1, instructions: vec![WaterSector::new(1, time, 15 * 3600)] });
        // Call the execute function
        ws.execute_active_mode(time_provider.now()).await;

        {
            // Verify watering state
            if should_water {
                assert_ne!(ws.water_state.state, WateringState::Idle, "Expected watering to start.");
            } else {
                assert_eq!(ws.water_state.state, WateringState::Idle, "Expected no watering outside timeframe.");
            }

            // Reset state
            ws.water_state.set_idle();
        }
    }
}

#[test]
fn watering_with_interrupts() {
    let allowed_timeframe = AllowedTimeframe::new(22, 8);
    let now = parse_datetime_to_utc_timestamp("2024-11-29T17:00:00+00:00", "%Y-%m-%dT%H:%M:%S%z").unwrap();
    let mock_db = Some(Arc::new(MockDatabase::new()));
    let mut ws = set_ws0(now, Some(ModeIdx::Wizard), mock_db).unwrap();
    ws.water_state.timeframe = allowed_timeframe;

    let sec = WaterSector::new(1, now, 30 * 3600);
    let cycle = Cycle { id: 1, instructions: vec![sec] };

    ws.water_state.start_cycle(cycle);
    ws.water_state.state = WateringState::Activating(sec);
    ws.water_state.handle_environmental_signal(EnvironmentalSignal::RainStart);

    assert_eq!(ws.water_state.state, WateringState::Idle);
}

#[tokio::test]
async fn run_watering_system_fast_forward() {
    let now = chrono::Utc::now().timestamp();
    let (app_state, mut ws) = set_app_and_ws(now, Some(ModeIdx::Wizard)).unwrap();
    let time_provider = ws.time_provider.clone();
    let allowed_timeframe = AllowedTimeframe::new(22, 8); // 10 PM to 6 AM
    ws.water_state.timeframe = allowed_timeframe;
    start_log(Some(time_provider.clone()));

    // Simulation parameters
    let simulation_duration_seconds = 6 * 24 * 3600; // 7 days

    let sectors = load_sectors_into_hashmap(vec![
        SectorInfo::build(1, 2.5, 1.5, 30 * 60, 0., 5.),
        SectorInfo::build(2, 12., 2., 40 * 60, 0., 4.),
    ]);
    ws.water_state.sectors = sectors;

    let wizard_schedule = Schedule::new(vec![ScheduleEntry {
        schedule_type: ScheduleType::Date(sod(now)),
        start_times: vec![
            WaterSector::new(1, sod(now) + (22 * 3600), 30 * 60), // Sector 1: 10 PM start
            WaterSector::new(2, sod(now) + (23 * 3600), 40 * 60), // Sector 2: 11 PM start
        ],
    }]);
    let auto_schedule = Schedule::new(vec![ScheduleEntry {
        schedule_type: ScheduleType::Weekday(chrono::Utc::now().weekday()),
        start_times: vec![WaterSector::new(1, sod(now) + (8 * 3600), 60 * 60)], // Sector 1: 8 AM start
    }]);

    ws.water_state.mode_wizard.schedule = wizard_schedule;
    ws.water_state.mode_auto.schedule = auto_schedule;

    // TODO: now it is in Wizard Mode.  Make a variant of the test to switch modes
    // let mut active_mode = app_state.watering_system.active_mode.write().await;
    // let auto_mode = app_state.watering_system.auto_mode.read().await.clone();
    // *active_mode = ModeEnum::Auto(auto_mode);
    assert_eq!(ws.water_state.active_mode, ModeIdx::Wizard, "Active mode should be Wizard Mode.");

    // Broadcast channel for control signals
    _ = run_watering_system(
        app_state.clone(),
        Some(ModeIdx::Wizard),
        Some(now + simulation_duration_seconds),
        Some(&mut ws),
    )
    .await;

    // Validate the results
    for sector in ws.water_state.sectors.values() {
        assert!(sector.progress > 0.0, "Sector {} should have positive progress after simulation.", sector.id);
    }
    assert!(ws.water_state.cycle.is_some(), "Cycle should be active after simulation.");
}
