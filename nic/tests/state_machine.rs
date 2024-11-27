use chrono::Duration;
use nic::watering::{
    ds::{Cycle, SectorInfo, WateringState},
    schedule::AllowedTimeframe,
    state_machine::WateringStateMachine,
    watering_system::load_sectors,
};
use test_utilities::common::set_app_state1;

#[test]
fn test_start_cycle() {
    // let timeframe = AllowedTimeframe {
    //     start: NaiveTime::from_hms_opt(6, 0, 0).unwrap(),
    //     end: NaiveTime::from_hms_opt(22, 0, 0).unwrap(),
    // };
    let mut state_machine = WateringStateMachine::new();

    let cycle = Cycle {
        id: 1,
        instructions: vec![(1, Duration::minutes(30))],
    };
    state_machine.start_cycle(cycle.clone());

    assert_eq!(state_machine.cycle, Some(cycle));
    assert_eq!(state_machine.current_instruction, 0);
    assert_eq!(state_machine.state, WateringState::Idle);
}

#[tokio::test]
async fn test_scheduler_triggers_modes() {
    let app_state = set_app_state1().await;
    let now = chrono::Local::now().time();

    let timeframe = AllowedTimeframe {
        start: now,
        end: now + chrono::Duration::hours(1),
    };
    *app_state.watering_system.timeframe.write().await = timeframe;

    let sectors = load_sectors(vec![SectorInfo {
        id: 1,
        weekly_target: 2.5,
        sprinkler_debit: 1.0,
        percolation_rate: 0.5,
        max_duration: chrono::Duration::minutes(30),
        progress: 0.,
    }]);
    *app_state.watering_system.sectors.write().await = sectors;

    // Initialize Auto Mode with a cycle
    {
        let mut auto_mode = app_state.watering_system.auto_mode.write().await;
        auto_mode.cycle = Cycle {
            id: 1,
            instructions: vec![(1, chrono::Duration::minutes(30))],
        };
    }

    // Simulate `run_watering_system` loop for scheduling
    let task = tokio::spawn({
        let app_state = app_state.clone();
        async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
            for _ in 0..5 {
                interval.tick().await;

                let now = chrono::Local::now().time();

                // Always update Wizard Mode data
                let mut wizard_mode = app_state.watering_system.wizard_mode.write().await;
                // Perform mode-specific scheduling logic for Wizard Mode
                if let Some(next_start) = wizard_mode.calculate_next_start(now, timeframe) {
                    if now >= next_start {
                        wizard_mode
                            .execute(&app_state.watering_system, now, &app_state.db)
                            .await;
                    }
                }
            }
        }
    });
    let _x = task.await;

    // tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    assert!(
        app_state
            .watering_system
            .sectors
            .read()
            .await
            .get(&1)
            .unwrap()
            .progress
            > 0.
    );
}
