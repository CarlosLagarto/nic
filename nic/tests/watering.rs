use async_trait::async_trait;
use chrono::NaiveTime;
use mockall::mock;
use nic::{
    sensors::interface::SensorController,
    watering::{
        ds::{Cycle, EnvironmentalSignal, SectorInfo, WateringState},
        schedule::AllowedTimeframe,
        watering_system::load_sectors,
    },
};
use std::time::Duration;
use test_utilities::common::{set_app_state, set_app_state_and_controller};
use tokio::time::sleep;

mock! {
    pub SensorController {}

    #[async_trait]
    impl SensorController for SensorController {
        async fn activate_sector(&self, sector: u32);
        async fn deactivate_sector(&self, sector: u32);
    }
}

#[tokio::test]
async fn test_watering_at_right_times() {
    let (app_state, _controller) = set_app_state_and_controller().await;

    // Define allowed timeframe: 6 AM to 10 PM
    let allowed_timeframe = AllowedTimeframe {
        start: NaiveTime::from_hms_opt(6, 0, 0).unwrap(),
        end: NaiveTime::from_hms_opt(22, 0, 0).unwrap(),
    };

    // Set up WizardMode with sectors
    {
        *app_state.watering_system.timeframe.write().await = allowed_timeframe;

        let sectors = load_sectors(vec![
            SectorInfo {
                id: 1,
                sprinkler_debit: 1.5,
                weekly_target: 10.0,
                percolation_rate: 5.0,
                max_duration: chrono::Duration::minutes(30),
                progress: 0.,
            },
            SectorInfo {
                id: 2,
                sprinkler_debit: 2.0,
                weekly_target: 12.0,
                percolation_rate: 4.0,
                max_duration: chrono::Duration::minutes(40),
                progress: 0.,
            },
        ]);
        *app_state.watering_system.sectors.write().await = sectors;
    }

    // Simulate watering execution at various times
    let test_cases = vec![
        (NaiveTime::from_hms_opt(5, 30, 0).unwrap(), false), // Before allowed timeframe
        (NaiveTime::from_hms_opt(7, 0, 0).unwrap(), true),   // Within allowed timeframe
        (NaiveTime::from_hms_opt(22, 30, 0).unwrap(), false), // After allowed timeframe
    ];

    for (time, should_water) in test_cases {
        println!("Testing for time: {:?}", time);

        {
            // Mock the current time
            let mut state_machine = app_state.watering_system.state_machine.write().await;

            // Start a new cycle
            state_machine.start_cycle(Cycle {
                id: 1,
                instructions: vec![(1, chrono::Duration::minutes(15))],
            });
        }
        let mut wizard_mode = app_state.watering_system.wizard_mode.write().await;

        // Call the execute function
        wizard_mode
            .execute(&app_state.watering_system, time, &app_state.db)
            .await;

        {
            let mut state_machine = app_state.watering_system.state_machine.write().await;
            // Verify watering state
            if should_water {
                assert_ne!(
                    state_machine.state,
                    WateringState::Idle,
                    "Expected watering to start."
                );
            } else {
                assert_eq!(
                    state_machine.state,
                    WateringState::Idle,
                    "Expected no watering outside timeframe."
                );
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

    let cycle = Cycle {
        id: 1,
        instructions: vec![(1, chrono::Duration::minutes(30))],
    };

    state_machine.start_cycle(cycle);

    sleep(Duration::from_secs(1)).await;

    let mut wizard_mode = app_state.watering_system.wizard_mode.write().await;

    wizard_mode.handle_signal(EnvironmentalSignal::RainStart, &mut *state_machine);

    sleep(Duration::from_secs(1)).await;

    assert_eq!(state_machine.state, WateringState::Idle);
}
