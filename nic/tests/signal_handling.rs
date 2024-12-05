use std::sync::Arc;

use nic::{
    test::utils::{mock_db::MockDatabase, set_ws0},
    utils::sod,
    watering::{
        ds::{Cycle, EnvironmentalSignal, WaterSector, WateringState},
        modes::ModeIdx,
    },
};

#[test]
fn signal_handling() {
    let ref_time = sod(chrono::Utc::now().timestamp());
    let mock_db = Some(Arc::new(MockDatabase::new()));
    let mut ws = set_ws0(ref_time, Some(ModeIdx::Wizard), mock_db).unwrap();

    ws.water_state.start_cycle(Cycle { id: 1, instructions: vec![WaterSector::new(1, ref_time, 30 * 3600)] });

    ws.water_state.handle_environmental_signal(EnvironmentalSignal::RainStart);

    assert_eq!(ws.water_state.state, WateringState::Idle);

    ws.water_state.handle_environmental_signal(EnvironmentalSignal::RainStop);

    assert!(ws.water_state.cycle.is_some());
}

#[test]
fn weather_signal_handling_all_states() {
    let ref_time = sod(chrono::Utc::now().timestamp());
    let mock_db = Some(Arc::new(MockDatabase::new()));
    let mut ws = set_ws0(ref_time, Some(ModeIdx::Wizard), mock_db).unwrap();

    let duration = 30 * 3600;
    let sec = WaterSector::new(1, ref_time, duration);
    ws.water_state.start_cycle(Cycle { id: 1, instructions: vec![sec] });
    ws.water_state.state = WateringState::Activating(sec);

    ws.water_state.handle_environmental_signal(EnvironmentalSignal::RainStart);
    assert_eq!(ws.water_state.state, WateringState::Idle); // Paused due to rain
    assert!(ws.water_state.mode_wizard.paused_state.is_some());

    ws.water_state.handle_environmental_signal(EnvironmentalSignal::RainStop);
    assert!(ws.water_state.mode_wizard.paused_state.is_none()); // Paused state cleared
    assert!(ws.water_state.cycle.is_some()); // Cycle restored
    assert_eq!(ws.water_state.state, WateringState::Activating(sec)); // Resume properly

    ws.water_state.state = WateringState::Watering(sec); // Set state to watering
    ws.water_state.handle_environmental_signal(EnvironmentalSignal::HighWind);
    assert_eq!(ws.water_state.state, WateringState::Idle); // Paused due to high wind
    assert!(ws.water_state.mode_wizard.paused_state.is_some());

    ws.water_state.handle_environmental_signal(EnvironmentalSignal::LowWind);
    assert!(ws.water_state.mode_wizard.paused_state.is_none());
    assert!(ws.water_state.cycle.is_some());
    assert_eq!(ws.water_state.state, WateringState::Watering(sec)); // Resume properly

    ws.water_state.state = WateringState::Activating(sec);
    ws.water_state.handle_environmental_signal(EnvironmentalSignal::RainStart);
    assert_eq!(ws.water_state.state, WateringState::Idle); // Paused from activating state
    assert!(ws.water_state.mode_wizard.paused_state.is_some());

    ws.water_state.handle_environmental_signal(EnvironmentalSignal::RainStop);
    assert!(ws.water_state.mode_wizard.paused_state.is_none());
    assert!(ws.water_state.cycle.is_some());
    assert_eq!(ws.water_state.state, WateringState::Activating(sec)); // Resume properly
}

#[test]
fn signal_handling_high_wind_and_low_wind() {
    let ref_time = sod(chrono::Utc::now().timestamp());
    let mock_db = Some(Arc::new(MockDatabase::new()));
    let mut ws = set_ws0(ref_time, Some(ModeIdx::Wizard), mock_db).unwrap();

    let duration = 30 * 3600;
    let sec = WaterSector::new(1, ref_time, duration);
    ws.water_state.start_cycle(Cycle { id: 1, instructions: vec![sec] });
    ws.water_state.state = WateringState::Watering(sec);

    ws.water_state.handle_environmental_signal(EnvironmentalSignal::HighWind);
    assert_eq!(ws.water_state.state, WateringState::Idle); // Irrigation paused
    assert!(ws.water_state.mode_wizard.paused_state.is_some());

    ws.water_state.handle_environmental_signal(EnvironmentalSignal::LowWind);
    assert!(ws.water_state.cycle.is_some());
    assert_eq!(ws.water_state.state, WateringState::Watering(sec)); // Resumes watering state
}

#[test]
fn signal_handling_high_wind() {
    let ref_time = sod(chrono::Utc::now().timestamp());
    let mock_db = Some(Arc::new(MockDatabase::new()));
    let mut ws = set_ws0(ref_time, Some(ModeIdx::Wizard), mock_db).unwrap();
    let sm = &mut ws.water_state;

    let duration = 30 * 3600;
    let sec = WaterSector::new(1, ref_time, duration);
    sm.start_cycle(Cycle { id: 1, instructions: vec![sec] });
    sm.state = WateringState::Watering(sec);

    sm.handle_environmental_signal(EnvironmentalSignal::HighWind);

    assert_eq!(sm.state, WateringState::Idle); // Paused from watering state
    assert!(sm.mode_wizard.paused_state.is_some());
    assert!(sm.cycle.is_none());
}
