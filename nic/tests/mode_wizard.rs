use chrono::{Duration, NaiveTime};
use nic::watering::ds::{SectorInfo, WateringState};
use nic::watering::mode_wizard::ModeWizard;
use nic::watering::schedule::{AllowedTimeframe, Schedule};
use nic::watering::watering_system::WateringSystem;
use std::sync::Arc;
use test_utilities::common::{
    mock_db::MockDatabase,
    mock_sector::mock_sector,
    mock_sensors::{set_sensor_controller0, MockSensorController},
};

#[test]
fn test_calculate_irrigation_time() {
    let sector = mock_sector(1, 2.5, 1.0, Duration::minutes(30));
    let schedule = Schedule::new(vec![]); // Create an empty schedule for the wizard mode
    let wizard = ModeWizard::new(schedule);

    // No progress yet
    let result = wizard.calculate_irrigation_time(&sector);
    assert_eq!(result, Some(Duration::minutes(30))); // 2.5 cm at 1.0 cm/hour
}

#[tokio::test]
async fn test_execute_wizard_mode() {
    let mock_db = Arc::new(MockDatabase::new());
    let mock_controller = set_sensor_controller0();
    let watering_system = WateringSystem::new(mock_controller.clone(), mock_db.clone()).await.unwrap();

    // Mock sectors with progress and targets
    let mut sectors = watering_system.sectors.write().await;
    sectors.insert(
        1,
        SectorInfo {
            id: 1,
            sprinkler_debit: 1.0,
            percolation_rate: 0.5,
            max_duration: Duration::minutes(30),
            weekly_target: 2.5,
            progress: 1.0, // Remaining: 1.5
        },
    );
    sectors.insert(
        2,
        SectorInfo {
            id: 2,
            sprinkler_debit: 0.8,
            percolation_rate: 0.6,
            max_duration: Duration::minutes(20),
            weekly_target: 1.8,
            progress: 0.5, // Remaining: 1.3
        },
    );
    drop(sectors);

    let schedule = Schedule::new(vec![]); // Create an empty schedule for the wizard mode
    let mut wizard_mode = ModeWizard::new(schedule);

    let current_date = chrono::NaiveDate::from_ymd_opt(2023, 11, 25).unwrap();
    wizard_mode.execute(&watering_system, current_date, &mock_db).await;

    // Assert state transitions
    let sm = watering_system.state_machine.read().await;
    assert!(sm.cycle.is_some()); // A cycle should be active
    assert_eq!(sm.state, WateringState::Activating(1)); // First sector activation
}

#[tokio::test]
async fn test_daily_et_integration() {
    let mock_db = Arc::new(MockDatabase::new());
    let mock_controller = Arc::new(MockSensorController::new());
    let watering_system =
        Arc::new(WateringSystem::new(mock_controller.clone(), mock_db.clone()).await.unwrap());

    // Initial daily adjustments
    let mut wizard_mode = watering_system.wizard_mode.write().await;
    wizard_mode.handle_daily_adjustments(&watering_system, &mock_db).await;

    // Assert progress recalculations
    let sectors = watering_system.sectors.read().await;
    for sector in sectors.values() {
        assert!(
            sector.progress <= sector.weekly_target,
            "Progress for sector {} exceeded its weekly target: {:.2} > {:.2}",
            sector.id,
            sector.progress,
            sector.weekly_target
        );
    }

    // Assert schedule recalculations
    let schedule = &wizard_mode.schedule.entries;
    assert!(
        !schedule.is_empty(),
        "Wizard mode schedule should not be empty after daily adjustments."
    );
}

#[test]
fn test_calculate_deep_watering_schedule() {
    let sectors = vec![
        SectorInfo {
            id: 1,
            sprinkler_debit: 1.0,
            percolation_rate: 0.5,
            max_duration: Duration::minutes(40),
            weekly_target: 3.0,
            progress: 0.5,
        },
        SectorInfo {
            id: 2,
            sprinkler_debit: 0.8,
            percolation_rate: 0.6,
            max_duration: Duration::minutes(30),
            weekly_target: 2.5,
            progress: 0.3,
        },
    ];
    let timeframe = AllowedTimeframe {
        start: NaiveTime::from_hms_opt(6, 0, 0).unwrap(),
        end: NaiveTime::from_hms_opt(20, 0, 0).unwrap(),
    };
    let schedule = Schedule::new(vec![]);
    let wizard = ModeWizard::new(schedule);
    let schedule = wizard.calculate_deep_watering_schedule(
        &sectors, timeframe, 3.0, // Weekly target
        0.4, // ET
        5,   // Days remaining
    );

    assert_eq!(schedule.len(), 2);
    assert_eq!(schedule[0].0, NaiveTime::from_hms_opt(6, 0, 0).unwrap()); // First cycle
    assert!(schedule[0].1.len() > 0);
}
