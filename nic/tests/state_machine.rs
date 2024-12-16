use nic::{
    test::utils::{mock_db::MockDatabase, set_ws0},
    utils::{ux_ts_to_string, load_sectors_into_hashmap, sod},
    watering::{
        ds::{DailyPlan, SectorInfo, WaterSector},
        modes::Mode,
    },
};
use std::sync::Arc;

#[tokio::test]
async fn scheduler_triggers_auto_mode() {
    let now = chrono::Utc::now().timestamp();
    let mock_db = Some(Arc::new(MockDatabase::new()));
    let mut ws = set_ws0(now, Some(Mode::Auto), mock_db).unwrap();
    let time_provider = ws.time_provider.clone();

    let sectors = load_sectors_into_hashmap(vec![SectorInfo::build(1, 2.5, 1., 30 * 60, 0., 0.5, 0)]);
    ws.sm.sectors = sectors;

    let base_time = sod(now);
    let sec_start_time = base_time + (22 * 3600); // 10:00 PM today

    println!("Sector start: {}", ux_ts_to_string(sec_start_time));
    let sec = WaterSector::new(1, sec_start_time, 30 * 60); // Sector 1: Starts 1 minute from now, 30 minutes duration

    let daily_plan = DailyPlan(vec![sec]);
    ws.sm.mode_auto.daily_plan = vec![daily_plan];
    // Simulate `run_watering_system` loop for scheduling
    time_provider.set(sec_start_time - 1); // Start simulation slightly before the schedule
    for _ in 0..5 {
        println!("Simulated time: {}", ux_ts_to_string(time_provider.now()));
        // Execute Auto Mode if within timeframe
        let now = time_provider.now();
        if ws.sm.timeframe.is_within(now) {
            ws.sm.update(now);
        }
        time_provider.advance_time(1).await;
    }

    // Validate the state machine
    assert!(ws.sm.cycle.is_some(), "Cycle should be active in Auto Mode.");
    assert_eq!(ws.sm.cycle.as_ref().unwrap().daily_plan.0.len(), 1, "Cycle should contain one instruction.");
    assert_eq!(
        ws.sm.cycle.as_ref().unwrap().daily_plan.0[0],
        sec,
        "Cycle should target sector 1 with the correct duration."
    );
}

#[tokio::test]
async fn scheduler_triggers_wizard_mode() {
    let now = chrono::Utc::now().timestamp();
    let mock_db = Some(Arc::new(MockDatabase::new()));
    let mut ws = set_ws0(now, Some(Mode::Wizard), mock_db).unwrap();
    let time_provider = ws.time_provider.clone();

    let base_time = sod(now);
    let sec_start_time = base_time + (22 * 3600); // 10:00 PM today

    let sec = WaterSector::new(1, sec_start_time, 30 * 60);
    let daily_plan = DailyPlan(vec![sec]);
    ws.sm.mode_wizard.daily_plan = vec![daily_plan];
    time_provider.set(sec_start_time - 1); // Start simulation slightly before the schedule
    for _ in 0..5 {
        ws.sm.update(time_provider.now());
        time_provider.advance_time(1).await;
    }
    assert!(ws.sm.cycle.is_some(), "Cycle should be active in Wizard Mode.");
    assert_eq!(ws.sm.cycle.as_ref().unwrap().daily_plan.0.len(), 1, "Cycle should contain one instruction.");
    assert_eq!(
        ws.sm.cycle.as_ref().unwrap().daily_plan.0[0],
        sec,
        "Cycle should target sector 1 with the correct duration."
    );
}
