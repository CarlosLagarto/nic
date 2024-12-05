use chrono::TimeZone;
use nic::test::utils::mock_db::MockDatabase;
use nic::test::utils::set_ws0;
use nic::time::TimeProvider;
use nic::utils::sod;
use nic::watering::ds::{SectorInfo, WaterSector, WateringState};
use nic::watering::modes::ModeIdx;
use nic::watering::schedule::{Schedule, ScheduleEntry, ScheduleType, WateringSchedule};
use std::sync::Arc;

#[test]
fn calculate_irrigation_time() {
    let sector = SectorInfo::build(1, 2.5, 1.0, 30 * 60, 1., 0.5);

    // No progress yet
    let result = WateringSchedule::calculate_irrigation_time(&sector);
    assert_eq!(result, Some(30 * 60)); // 1.5 cm at 1.0 cm/hour
}
#[tokio::test]
async fn execute_wizard_mode() {
    let current_date = chrono::Utc.with_ymd_and_hms(2023, 11, 25, 22, 0, 0).unwrap().timestamp(); // 6:00 AM UTC
    let mut ws = set_ws0(current_date, Some(ModeIdx::Wizard), None).unwrap();
    // Mock sectors with progress and targets
    ws.water_state.sectors.insert(1, SectorInfo::build(1, 1.8, 1.0, 30 * 60, 1., 0.5));
    ws.water_state.sectors.insert(2, SectorInfo::build(2, 2.5, 0.8, 20 * 60, 1., 0.5));

    // Set up a valid schedule for wizard mode
    let schedule_entries = vec![ScheduleEntry {
        schedule_type: ScheduleType::Date(sod(current_date)), // Schedule for the specific test date
        start_times: vec![
            WaterSector::new(1, current_date + 3600, 1800), // Sector 1, start at 7:00 AM UTC, 30 min duration
            WaterSector::new(2, current_date + 7200, 1200), // Sector 2, start at 8:00 AM UTC, 20 min duration
        ],
    }];
    let schedule = Schedule::new(schedule_entries);

    ws.water_state.mode_wizard.schedule = schedule;

    // Execute wizard mode
    ws.time_provider.advance_time(3600);
    let now = ws.time_provider.now();
    ws.execute_active_mode(now).await;

    // Assert state transitions
    assert!(ws.water_state.cycle.is_some()); // A cycle should be active
    assert_eq!(ws.water_state.cycle.as_ref().unwrap().instructions.len(), 2); // Two instructions in the cycle
    assert_eq!(ws.water_state.state, WateringState::Watering(WaterSector::new(1, current_date + 3600, 1800)));
    // The state machine should be in the Idle state
}

#[test]
fn handle_daily_adjustments() {
    let ref_time = sod(chrono::Utc::now().timestamp());
    let mock_db = Some(Arc::new(MockDatabase::new()));
    let mut ws = set_ws0(ref_time, Some(ModeIdx::Wizard), mock_db).unwrap();

    ws.water_state.sectors.insert(1, SectorInfo::build(1, 1.8, 1.0, 30 * 60, 1., 0.5));
    ws.water_state.sectors.insert(2, SectorInfo::build(2, 2.5, 0.8, 20 * 60, 1., 0.5));

    ws.water_state.do_daily_adjustments(ref_time, 0.5, 0.1);

    // Verify sector progress
    assert_eq!(ws.water_state.sectors[&1].progress, 0.6); // Adjusted for ET and rain
    assert_eq!(ws.water_state.sectors[&2].progress, 0.6);
}
