use chrono::Datelike;
use nic::{
    test::utils::{mock_db::MockDatabase, set_ws0},
    time::TimeProvider,
    utils::{display_from_ts, load_sectors_into_hashmap, sod},
    watering::{
        ds::{Cycle, SectorInfo, WaterSector, WateringState},
        modes::ModeIdx,
        schedule::{Schedule, ScheduleEntry, ScheduleType},
        water_state::WaterState,
    },
};
use std::sync::Arc;

#[test]
fn start_cycle() {
    let sectors = vec![SectorInfo::build(1, 2.5, 1., 30 * 60, 0., 0.5)];
    let mut state_machine = WaterState::new(None, sectors);
    let sec = WaterSector::new(1, 0, 30 * 3600);
    let cycle = Cycle { id: 1, instructions: vec![sec] };
    state_machine.start_cycle(cycle.clone());

    assert_eq!(state_machine.cycle, Some(cycle));
    assert_eq!(state_machine.current_instruction, 0);
    assert_eq!(state_machine.state, WateringState::Idle);
}

#[tokio::test]
async fn scheduler_triggers_auto_mode() {
    let now = chrono::Utc::now().timestamp();
    let mock_db = Some(Arc::new(MockDatabase::new()));
    let mut ws = set_ws0(now, Some(ModeIdx::Auto), mock_db).unwrap();
    let time_provider = ws.time_provider.clone();

    let sectors = load_sectors_into_hashmap(vec![SectorInfo::build(1, 2.5, 1., 30 * 60, 0., 0.5)]);
    ws.water_state.sectors = sectors;

    let base_time = sod(now);
    let sec_start_time = base_time + (22 * 3600); // 10:00 PM today
    let today_weekday = chrono::Utc::now().weekday(); // Dynamically get the current weekday

    println!("Sector start: {}", display_from_ts(sec_start_time));
    let sec = WaterSector::new(1, sec_start_time, 30 * 60);
    // Initialize Auto Mode with a schedule
    let schedule_entries = vec![ScheduleEntry {
        schedule_type: ScheduleType::Weekday(today_weekday),
        start_times: vec![sec], // Sector 1: Starts 1 minute from now, 30 minutes duration
    }];
    ws.water_state.mode_auto.schedule = Schedule::new(schedule_entries);
    // Simulate `run_watering_system` loop for scheduling
    time_provider.set(sec_start_time - 1); // Start simulation slightly before the schedule
    for _ in 0..5 {
        println!("Simulated time: {}", display_from_ts(time_provider.now()));
        // Execute Auto Mode if within timeframe
        let now = time_provider.now();
        if ws.water_state.timeframe.is_within(now, base_time) {
            ws.execute_active_mode(now).await;
        }
        time_provider.advance_time(1); // Simulate time passing
    }

    // Validate the state machine
    assert!(ws.water_state.cycle.is_some(), "Cycle should be active in Auto Mode.");
    assert_eq!(ws.water_state.cycle.as_ref().unwrap().instructions.len(), 1, "Cycle should contain one instruction.");
    assert_eq!(
        ws.water_state.cycle.as_ref().unwrap().instructions[0],
        sec,
        "Cycle should target sector 1 with the correct duration."
    );
}

#[tokio::test]
async fn scheduler_triggers_wizard_mode() {
    let now = chrono::Utc::now().timestamp();
    let mock_db = Some(Arc::new(MockDatabase::new()));
    let mut ws = set_ws0(now, Some(ModeIdx::Wizard), mock_db).unwrap();
    let time_provider = ws.time_provider.clone();

    let base_time = sod(now);
    let sec_start_time = base_time + (22 * 3600); // 10:00 PM today

    // Initialize Wizard Mode with a schedule
    let sec = WaterSector::new(1, sec_start_time, 30 * 60);
    let schedule_entries = vec![ScheduleEntry {
        schedule_type: ScheduleType::Date(sod(now)), // Today's start of day
        start_times: vec![sec],                      // Sector 1, 30 minutes duration
    }];
    ws.water_state.mode_wizard.schedule = Schedule::new(schedule_entries);
    // Simulate time passing for Wizard Mode
    time_provider.set(sec_start_time - 1); // Start simulation slightly before the schedule
    for _ in 0..5 {
        ws.execute_active_mode(time_provider.now()).await;
        time_provider.advance_time(1);
    }

    // Validate the state machine
    assert!(ws.water_state.cycle.is_some(), "Cycle should be active in Wizard Mode.");
    assert_eq!(ws.water_state.cycle.as_ref().unwrap().instructions.len(), 1, "Cycle should contain one instruction.");
    assert_eq!(
        ws.water_state.cycle.as_ref().unwrap().instructions[0],
        sec,
        "Cycle should target sector 1 with the correct duration."
    );
}
