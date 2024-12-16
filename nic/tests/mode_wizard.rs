use chrono::{TimeZone, Utc};
use nic::test::utils::mock_db::MockDatabase;
use nic::test::utils::set_ws0;
use nic::watering::ds::{DailyPlan, SectorInfo, WaterSector};
use nic::watering::modes::Mode;
use nic::watering::state_machine::SMState;
use std::sync::Arc;

#[tokio::test]
async fn execute_wizard_mode() {
    let current_date = Utc.with_ymd_and_hms(2023, 11, 25, 22, 0, 0).unwrap().timestamp(); // 6:00 AM UTC
    let mut ws = set_ws0(current_date, Some(Mode::Wizard), None).unwrap();
    // Mock sectors with progress and targets
    ws.sm.sectors.insert(1, SectorInfo::build(1, 1.8, 1.0, 30 * 60, 1., 0.5, 0));
    ws.sm.sectors.insert(2, SectorInfo::build(2, 2.5, 0.8, 20 * 60, 1., 0.5, 0));

    // Set up a valid schedule for wizard mode
    let daily_plan = DailyPlan(vec![
        WaterSector::new(1, current_date + 3600, 1800), // Sector 1, start at 7:00 AM UTC, 30 min duration
        WaterSector::new(2, current_date + 7200, 1200), // Sector 2, start at 8:00 AM UTC, 20 min duration
    ]);
    ws.sm.mode_wizard.daily_plan = vec![daily_plan];

    // Execute wizard mode
    ws.time_provider.advance_time(3600).await;
    let now = ws.time_provider.now();
    ws.sm.update(now);

    // Assert state transitions
    assert!(ws.sm.cycle.is_some()); // A cycle should be active
    assert_eq!(ws.sm.cycle.as_ref().unwrap().daily_plan.0.len(), 2); // Two instructions in the cycle
    assert_eq!(ws.sm.state, SMState::Watering(WaterSector::new(1, current_date + 3600, 1800)));
    // The state machine should be in the Idle state
}

#[test]
fn handle_daily_adjustments() {
    let ref_time = Utc.with_ymd_and_hms(2024, 12, 10, 22, 0, 0).unwrap().timestamp(); // 6:00 AM UTC
    let mock_db = Some(Arc::new(MockDatabase::new()));
    let mut ws = set_ws0(ref_time, Some(Mode::Wizard), mock_db).unwrap();

    ws.sm.sectors.insert(1, SectorInfo::build(1, 1.8, 1.0, 30 * 60, 1., 0., 0));
    ws.sm.sectors.insert(2, SectorInfo::build(2, 2.5, 0.8, 20 * 60, 1., 0., 0));

    ws.sm.do_daily_adjustments(ref_time, 0.5, 0.1);

    // Verify sector progress
    assert_eq!(ws.sm.sectors[&1].progress, 0.6); // Adjusted for ET and rain
    assert_eq!(ws.sm.sectors[&2].progress, 0.6);
}
