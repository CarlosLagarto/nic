use chrono::NaiveTime;

use async_trait::async_trait;
use mockall::mock;
use nic::watering::{
    ds::{Cycle, SectorInfo, WateringState},
    interface::SensorController,
    schedule::AllowedTimeframe,
};

#[path = "common/mod.rs"]
mod common;
use crate::common::{
    mock_db::{new_with_mock, MockDatabase},
    mock_sensors::set_sensor_controller,
};

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
    let db = MockDatabase::new();
    let controller = set_sensor_controller();
    let app_state = new_with_mock(db, controller.clone()).await;

    // Define allowed timeframe: 6 AM to 10 PM
    let allowed_timeframe = AllowedTimeframe {
        start: NaiveTime::from_hms_opt(6, 0, 0).unwrap(),
        end: NaiveTime::from_hms_opt(22, 0, 0).unwrap(),
    };

    // Set up WizardMode with sectors
    {
        let mut wizard_mode = app_state.watering_system.wizard_mode.write().await;
        wizard_mode.timeframe = allowed_timeframe.clone();
        wizard_mode.sectors = vec![
            SectorInfo {
                id: 1,
                sprinkler_debit: 1.5,
                weekly_target: 10.0,
                percolation_rate: 5.0,
                max_duration: chrono::Duration::minutes(30),
            },
            SectorInfo {
                id: 2,
                sprinkler_debit: 2.0,
                weekly_target: 12.0,
                percolation_rate: 4.0,
                max_duration: chrono::Duration::minutes(40),
            },
        ];
    }

    // Simulate watering execution at various times
    let test_cases = vec![
        (NaiveTime::from_hms_opt(5, 30, 0).unwrap(), false), // Before allowed timeframe
        (NaiveTime::from_hms_opt(7, 0, 0).unwrap(), true),   // Within allowed timeframe
        (NaiveTime::from_hms_opt(22, 30, 0).unwrap(), false), // After allowed timeframe
    ];

    for (time, should_water) in test_cases {
        println!("Testing for time: {:?}", time);

        // Mock the current time
        let mut state_machine = app_state.watering_system.state_machine.write().await;

        // Start a new cycle
        state_machine.start_cycle(Cycle {
            id: 1,
            instructions: vec![(1, chrono::Duration::minutes(15))],
        });

        let mut wizard_mode = app_state.watering_system.wizard_mode.write().await;

        // Call the execute function
        wizard_mode
            .execute(&mut *state_machine, app_state.db.clone(), time, &controller)
            .await;

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
