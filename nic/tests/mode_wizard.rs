use chrono::TimeZone;
use nic::utils::sod;
use nic::watering::ds::{SectorInfo, WateringState};
use nic::watering::mode_wizard::ModeWizard;
use nic::watering::schedule::{Schedule, ScheduleEntry, ScheduleType, WateringSchedule};
use nic::watering::watering_system::WateringSystem;
use std::sync::Arc;
use test_utilities::common::{mock_db::MockDatabase, mock_sensors::set_sensor_controller0};

#[test]
fn test_calculate_irrigation_time() {
    let sector = SectorInfo::build(1, 2.5, 1.0, 30 * 60, 1., 0.5);

    // No progress yet
    let result = WateringSchedule::calculate_irrigation_time(&sector);
    assert_eq!(result, Some(30 * 60)); // 1.5 cm at 1.0 cm/hour
}
#[tokio::test]
async fn test_execute_wizard_mode() {
    let mock_db = Arc::new(MockDatabase::new());
    let mock_controller = set_sensor_controller0();
    let watering_system = WateringSystem::new(mock_controller.clone(), mock_db.clone()).await.unwrap();

    // Mock sectors with progress and targets
    let mut sectors = watering_system.sectors.write().await;
    sectors.insert(1, SectorInfo::build(1, 1.8, 1.0, 30 * 60, 1., 0.5));
    sectors.insert(2, SectorInfo::build(2, 2.5, 0.8, 20 * 60, 1., 0.5));
    drop(sectors);

    // Set up a valid schedule for wizard mode
    let current_date = chrono::Utc.with_ymd_and_hms(2023, 11, 25, 6, 0, 0).unwrap().timestamp(); // 6:00 AM UTC
    let schedule_entries = vec![ScheduleEntry {
        schedule_type: ScheduleType::Date(sod(current_date)), // Schedule for the specific test date
        start_times: vec![
            (1, current_date + 3600, 1800), // Sector 1, start at 7:00 AM UTC, 30 min duration
            (2, current_date + 7200, 1200), // Sector 2, start at 8:00 AM UTC, 20 min duration
        ],
    }];

    let schedule = Schedule::new(schedule_entries);
    let mut wizard_mode = ModeWizard::new(schedule);

    // Execute wizard mode
    wizard_mode.execute(&watering_system, current_date + 3600, &mock_db).await;

    // Assert state transitions
    let sm = watering_system.state_machine.read().await;
    assert!(sm.cycle.is_some()); // A cycle should be active
    assert_eq!(sm.cycle.as_ref().unwrap().instructions.len(), 1); // Two instructions in the cycle
    assert_eq!(sm.state, WateringState::Activating(1)); // The state machine should be in the Idle state
}

#[tokio::test]
async fn test_handle_daily_adjustments() {
    let mut mock_db = MockDatabase::new();
    let ref_time = sod(chrono::Utc::now().timestamp());
    mock_db.et_data.insert(ref_time, 0.5); // 0.5 cm ET
    mock_db.rain_data.insert(ref_time, 0.1); // 0.1 mm rain

    let mock_db = Arc::new(mock_db);
    let mock_controller = set_sensor_controller0();
    let watering_system = WateringSystem::new(mock_controller, mock_db.clone()).await.unwrap();

    let mut wizard_mode = ModeWizard::new(Schedule::new(vec![]));

    // Mock sectors - code block to release write lock
    {
        let mut sectors = watering_system.sectors.write().await;
        sectors.insert(1, SectorInfo::build(1, 1.8, 1.0, 30 * 60, 1., 0.5));
        sectors.insert(2, SectorInfo::build(2, 2.5, 0.8, 20 * 60, 1., 0.5));
    }

    // Perform daily adjustments
    wizard_mode.handle_daily_adjustments(&watering_system, &mock_db, ref_time).await;

    // Verify sector progress
    let sectors = watering_system.sectors.read().await;
    assert_eq!(sectors[&1].progress, 0.6); // Adjusted for ET and rain
    assert_eq!(sectors[&2].progress, 0.6);
}
