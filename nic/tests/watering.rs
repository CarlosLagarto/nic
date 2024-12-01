use nic::{
    utils::{display_from_ts, parse_datetime_to_utc_timestamp, sod},
    watering::{
        ds::{Cycle, EnvironmentalSignal, SectorInfo, WateringState},
        schedule::{AllowedTimeframe, Schedule, ScheduleEntry, ScheduleType},
        watering_system::load_sectors_into_hashmap,
    },
};
use std::time::Duration;
use test_utilities::common::{set_app_state, set_app_state_and_controller};
use tokio::time::sleep;

#[tokio::test]
async fn test_watering_at_right_times() {
    let (app_state, _controller) = set_app_state_and_controller().await;

    let allowed_timeframe = AllowedTimeframe::new(22, 8);

     // Set up WizardMode with sectors and schedule
    {
        *app_state.watering_system.timeframe.write().await = allowed_timeframe;

        let sectors = load_sectors_into_hashmap(vec![
            SectorInfo::build(1, 2.5, 1.5, 30 * 60, 0., 5.),
            SectorInfo::build(2, 12., 2., 40 * 60, 0., 4.),
        ]);
        *app_state.watering_system.sectors.write().await = sectors;

        let now = parse_datetime_to_utc_timestamp("2024-11-29T17:00:00+00:00", "%Y-%m-%dT%H:%M:%S%z").unwrap();
        let schedule_entries = vec![ScheduleEntry {
            schedule_type: ScheduleType::Date(sod(now)), // Start of day for today
            start_times: vec![
                (1, sod(now) + (22 * 3600), 30 * 60), // Sector 1, start at 22:00 UTC, 30 mins duration
            ],
        }];
        let mut wizard_mode = app_state.watering_system.wizard_mode.write().await;
        wizard_mode.schedule = Schedule::new(schedule_entries);
    }

    // Simulate watering execution at various times
    let test_cases: Vec<(i64, bool)> = vec![
        (parse_datetime_to_utc_timestamp("2024-11-29T17:00:00+00:00", "%Y-%m-%dT%H:%M:%S%z").unwrap(), false), // Before allowed timeframe
        (parse_datetime_to_utc_timestamp("2024-11-29T22:00:00+00:00", "%Y-%m-%dT%H:%M:%S%z").unwrap(), true), // Within allowed timeframe
        (parse_datetime_to_utc_timestamp("2024-11-30T07:00:00+00:00", "%Y-%m-%dT%H:%M:%S%z").unwrap(), false), // After allowed timeframe
    ];

    for (time, should_water) in test_cases {
        println!("Testing for time: {:?}", display_from_ts(time));

        {
            let mut state_machine = app_state.watering_system.state_machine.write().await;
            state_machine.start_cycle(Cycle { id: 1, instructions: vec![(1, 15 * 3600)] });
        }
        let mut wizard_mode = app_state.watering_system.wizard_mode.write().await;

        // Call the execute function
        wizard_mode.execute(&app_state.watering_system, time, &app_state.db).await;

        {
            let mut state_machine = app_state.watering_system.state_machine.write().await;
            // Verify watering state
            if should_water {
                assert_ne!(state_machine.state, WateringState::Idle, "Expected watering to start.");
            } else {
                assert_eq!(state_machine.state, WateringState::Idle, "Expected no watering outside timeframe.");
            }

            // Reset state
            state_machine.state = WateringState::Idle;
            state_machine.cycle = None;
        }
    }
}

#[tokio::test]
async fn test_watering_with_interrupts() {
    let app_state = set_app_state().await;

    let mut state_machine = app_state.watering_system.state_machine.write().await;

    let cycle = Cycle { id: 1, instructions: vec![(1, 30 * 3600)] };

    state_machine.start_cycle(cycle);

    sleep(Duration::from_secs(1)).await;

    let mut wizard_mode = app_state.watering_system.wizard_mode.write().await;

    wizard_mode.handle_signal(EnvironmentalSignal::RainStart, &mut *state_machine);

    sleep(Duration::from_secs(1)).await;

    assert_eq!(state_machine.state, WateringState::Idle);
}
