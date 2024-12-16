use nic::{
    test::utils::{mock_db::MockDatabase, set_ws0},
    utils::sod,
    watering::{
        ds::{CtrlSignal, DailyPlan, WeatherSignal, WaterSector},
        modes::Mode,
    },
};
use std::sync::Arc;

#[test]
fn signal_handling() {
    let ref_time = sod(chrono::Utc::now().timestamp());
    let mock_db = Some(Arc::new(MockDatabase::new()));
    let mut ws = set_ws0(ref_time, Some(Mode::Wizard), mock_db).unwrap();

    let start_time = sod(ref_time) + (22 * 3600); //start at 22:00 UTC
    let daily_plan = DailyPlan(vec![
        WaterSector::new(1, start_time, 30 * 60), // Sector 1, , 30 mins duration
    ]);
    ws.sm.mode_wizard.daily_plan = vec![daily_plan];
    ws.sm.trans_watering(start_time);
    assert!(ws.sm.state.is_watering());
    ws.sm.handle_signal(CtrlSignal::Weather(WeatherSignal::RainStart), start_time + 2);

    assert!(ws.sm.state.is_paused());

    ws.sm.handle_signal(CtrlSignal::Weather(WeatherSignal::RainStop), start_time + 4);
    assert!(ws.sm.state.is_watering());
}

#[test]
fn weather_signal_handling_all_states() {
    let ref_time = sod(chrono::Utc::now().timestamp());
    let mock_db = Some(Arc::new(MockDatabase::new()));
    let mut ws = set_ws0(ref_time, Some(Mode::Wizard), mock_db).unwrap();

    let duration = 30 * 60;
    let start_time = ref_time + 22 * 3600;
    let sec = WaterSector::new(1, start_time, duration);
    let daily_plan = DailyPlan(vec![sec]);
    ws.sm.mode_wizard.daily_plan = vec![daily_plan];

    ws.sm.trans_watering(start_time);

    ws.sm.handle_signal(CtrlSignal::Weather(WeatherSignal::RainStart), start_time + 2);
    assert!(ws.sm.state.is_paused());

    ws.sm.handle_signal(CtrlSignal::Weather(WeatherSignal::RainStop), start_time + 4);
    assert!(ws.sm.state.is_watering());

    ws.sm.handle_signal(CtrlSignal::Weather(WeatherSignal::HighWind), start_time + 6);
    assert!(ws.sm.state.is_paused());

    ws.sm.handle_signal(CtrlSignal::Weather(WeatherSignal::LowWind), start_time + 8);
    assert!(ws.sm.state.is_watering());

    ws.sm.handle_signal(CtrlSignal::Weather(WeatherSignal::RainStart), start_time + 10);
    assert!(ws.sm.state.is_paused());

    ws.sm.handle_signal(CtrlSignal::Weather(WeatherSignal::RainStop), start_time + 12);
    assert!(ws.sm.state.is_watering());
}
