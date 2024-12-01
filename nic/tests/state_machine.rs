use chrono::Datelike;
use nic::{
    utils::{display_from_ts, sod},
    watering::{
        ds::{Cycle, SectorInfo, WateringState},
        schedule::{Schedule, ScheduleEntry, ScheduleType},
        state_machine::WateringStateMachine,
        watering_system::load_sectors_into_hashmap,
    },
};
use test_utilities::common::set_app_state1;

#[test]
fn test_start_cycle() {
    let mut state_machine = WateringStateMachine::new();

    let cycle = Cycle { id: 1, instructions: vec![(1, 30 * 3600)] };
    state_machine.start_cycle(cycle.clone());

    assert_eq!(state_machine.cycle, Some(cycle));
    assert_eq!(state_machine.current_instruction, 0);
    assert_eq!(state_machine.state, WateringState::Idle);
}

#[tokio::test]
async fn test_scheduler_triggers_auto_mode() {
    let app_state = set_app_state1().await;
    let sectors = load_sectors_into_hashmap(vec![SectorInfo::build(1, 2.5, 1., 30 * 60, 0., 0.5)]);
    *app_state.watering_system.sectors.write().await = sectors;

    let now = chrono::Utc::now().timestamp();
    let base_time = sod(now);
    let sec_start_time = base_time + (22 * 3600); // 10:00 PM today
    let today_weekday = chrono::Utc::now().weekday(); // Dynamically get the current weekday

    println!("Sector start: {}", display_from_ts(sec_start_time));
    // Initialize Auto Mode with a schedule
    let schedule_entries = vec![ScheduleEntry {
        schedule_type: ScheduleType::Weekday(today_weekday),
        start_times: vec![(1, sec_start_time, 30 * 60)], // Sector 1: Starts 1 minute from now, 30 minutes duration
    }];
    {
        let mut auto_mode = app_state.watering_system.auto_mode.write().await;
        auto_mode.schedule = Schedule::new(schedule_entries);
    }
    // Simulate `run_watering_system` loop for scheduling
    let task = tokio::spawn({
        let app_state = app_state.clone();
        async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
            let mut simulated_time = sec_start_time - 1; // Start simulation slightly before the schedule

            for _ in 0..5 {
                let timeframe = app_state.watering_system.timeframe.read().await;
                println!("Simulated time: {}", display_from_ts(simulated_time));
                interval.tick().await;
                // Execute Auto Mode if within timeframe
                if timeframe.is_within(simulated_time, base_time) {
                    let auto_mode = app_state.watering_system.auto_mode.write().await;
                    auto_mode.execute(&app_state.watering_system, &app_state.db, simulated_time).await;
                }
                simulated_time += 1; // Simulate time passing
            }
        }
    });

    let _ = task.await;

    // Validate the state machine
    let sm = app_state.watering_system.state_machine.read().await;
    assert!(sm.cycle.is_some(), "Cycle should be active in Auto Mode.");
    assert_eq!(sm.cycle.as_ref().unwrap().instructions.len(), 1, "Cycle should contain one instruction.");
    assert_eq!(
        sm.cycle.as_ref().unwrap().instructions[0],
        (1, 30 * 60),
        "Cycle should target sector 1 with the correct duration."
    );
}

#[tokio::test]
async fn test_scheduler_triggers_wizard_mode() {
    let app_state = set_app_state1().await;
    let sectors = load_sectors_into_hashmap(vec![SectorInfo::build(1, 2.5, 1., 30 * 60, 0., 0.5)]);
    *app_state.watering_system.sectors.write().await = sectors;


    let now = chrono::Utc::now().timestamp();
    let base_time = sod(now);
    let sec_start_time = base_time + (22 * 3600); // 10:00 PM today
    // Initialize Wizard Mode with a schedule
    let schedule_entries = vec![ScheduleEntry {
        schedule_type: ScheduleType::Date(sod(now)), // Today's start of day
        start_times: vec![(1, sec_start_time, 30 * 60)], // Sector 1, 30 minutes duration
    }];
    {
        let mut wizard_mode = app_state.watering_system.wizard_mode.write().await;
        wizard_mode.schedule = Schedule::new(schedule_entries);
    }

    // Simulate time passing for Wizard Mode
    let task = tokio::spawn({
        let app_state = app_state.clone();
        async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
            let mut simulated_time = sec_start_time - 1; // Start simulation slightly before the schedule
            for _ in 0..5 {
                interval.tick().await;
            
                let mut wizard_mode = app_state.watering_system.wizard_mode.write().await;
                wizard_mode.execute(&app_state.watering_system, simulated_time, &app_state.db).await;
                simulated_time += 1;
            }
        }
    });

    let _ = task.await;

    // Validate the state machine
    let sm = app_state.watering_system.state_machine.read().await;
    assert!(sm.cycle.is_some(), "Cycle should be active in Wizard Mode.");
    assert_eq!(sm.cycle.as_ref().unwrap().instructions.len(), 1, "Cycle should contain one instruction.");
    assert_eq!(
        sm.cycle.as_ref().unwrap().instructions[0],
        (1, 30 * 60),
        "Cycle should target sector 1 with the correct duration."
    );
}
