use nic::watering::ds::{Cycle, EnvironmentalSignal, WateringState};

use test_utilities::common::set_app_state;

#[tokio::test]
async fn test_signal_handling() {
    let app_state = set_app_state().await;
    let mut state_machine = app_state.watering_system.state_machine.write().await;

    state_machine.start_cycle(Cycle { id: 1, instructions: vec![(1, 30 * 3600)] });

    let mut wizard_mode = app_state.watering_system.wizard_mode.write().await;
    wizard_mode.handle_signal(EnvironmentalSignal::RainStart, &mut *state_machine);

    assert_eq!(state_machine.state, WateringState::Idle);

    wizard_mode.handle_signal(EnvironmentalSignal::RainStop, &mut *state_machine);
    assert!(state_machine.cycle.is_some());
}

#[tokio::test]
async fn test_weather_signal_handling_all_states() {
    let app_state = set_app_state().await;

    let mut state_machine = app_state.watering_system.state_machine.write().await;

    let duration = 30 * 3600;
    state_machine.start_cycle(Cycle { id: 1, instructions: vec![(1, duration)] });

    let mut wizard_mode = app_state.watering_system.wizard_mode.write().await;

    state_machine.state = WateringState::Activating(1);

    wizard_mode.handle_signal(EnvironmentalSignal::RainStart, &mut *state_machine);
    assert_eq!(state_machine.state, WateringState::Idle); // Paused due to rain
    assert!(wizard_mode.paused_state.is_some());

    wizard_mode.handle_signal(EnvironmentalSignal::RainStop, &mut *state_machine);
    assert!(wizard_mode.paused_state.is_none()); // Paused state cleared
    assert!(state_machine.cycle.is_some()); // Cycle restored
    assert_eq!(state_machine.state, WateringState::Activating(1)); // Resume properly

    state_machine.state = WateringState::Watering(1, duration); // Set state to watering
    wizard_mode.handle_signal(EnvironmentalSignal::HighWind, &mut *state_machine);
    assert_eq!(state_machine.state, WateringState::Idle); // Paused due to high wind
    assert!(wizard_mode.paused_state.is_some());

    wizard_mode.handle_signal(EnvironmentalSignal::LowWind, &mut *state_machine);
    assert!(wizard_mode.paused_state.is_none());
    assert!(state_machine.cycle.is_some());
    assert_eq!(state_machine.state, WateringState::Watering(1, duration)); // Resume properly

    state_machine.state = WateringState::Activating(1);
    wizard_mode.handle_signal(EnvironmentalSignal::RainStart, &mut *state_machine);
    assert_eq!(state_machine.state, WateringState::Idle); // Paused from activating state
    assert!(wizard_mode.paused_state.is_some());

    wizard_mode.handle_signal(EnvironmentalSignal::RainStop, &mut *state_machine);
    assert!(wizard_mode.paused_state.is_none());
    assert!(state_machine.cycle.is_some());
    assert_eq!(state_machine.state, WateringState::Activating(1)); // Resume properly
}

#[tokio::test]
async fn test_signal_handling_high_wind_and_low_wind() {
    let app_state = set_app_state().await;

    let mut state_machine = app_state.watering_system.state_machine.write().await;

    let duration = 30 * 3600;
    state_machine.start_cycle(Cycle { id: 1, instructions: vec![(1, duration)] });
    state_machine.state = WateringState::Watering(1, duration);

    let mut wizard_mode = app_state.watering_system.wizard_mode.write().await;

    wizard_mode.handle_signal(EnvironmentalSignal::HighWind, &mut *state_machine);
    assert_eq!(state_machine.state, WateringState::Idle); // Irrigation paused
    assert!(wizard_mode.paused_state.is_some());

    wizard_mode.handle_signal(EnvironmentalSignal::LowWind, &mut *state_machine);
    assert!(state_machine.cycle.is_some());
    assert_eq!(state_machine.state, WateringState::Watering(1, duration)); // Resumes watering state
}

#[tokio::test]
async fn test_signal_handling_high_wind() {
    let app_state = set_app_state().await;

    let mut state_machine = app_state.watering_system.state_machine.write().await;

    let duration = 30 * 3600;
    state_machine.start_cycle(Cycle { id: 1, instructions: vec![(1, duration)] });
    state_machine.state = WateringState::Watering(1, duration);

    let mut wizard_mode = app_state.watering_system.wizard_mode.write().await;

    wizard_mode.handle_signal(EnvironmentalSignal::HighWind, &mut *state_machine);

    assert_eq!(state_machine.state, WateringState::Idle); // Paused from watering state
    assert!(wizard_mode.paused_state.is_some());
    assert!(state_machine.cycle.is_none());
}
