use std::sync::Arc;

use chrono::Duration;

use crate::{
    db::mock::MockDatabase,
    watering::ds::{AppState, Cycle, EnvironmentalSignal, WateringState},
};

#[tokio::test]
async fn test_signal_handling() {
    let db = MockDatabase::new();
    let app_state = Arc::new(AppState::new_with_mock(db).await);
    let mut state_machine = app_state.watering_system.state_machine.write().await;

    state_machine.start_cycle(Cycle {
        id: 1,
        instructions: vec![(1, Duration::minutes(30))],
    });

    let mut wizard_mode = app_state.watering_system.wizard_mode.write().await;
    wizard_mode.handle_signal(EnvironmentalSignal::RainStart, &mut *state_machine);

    assert_eq!(state_machine.state, WateringState::Idle);

    wizard_mode.handle_signal(EnvironmentalSignal::RainStop, &mut *state_machine);
    assert!(state_machine.cycle.is_some());
}

#[tokio::test]
async fn test_weather_signal_handling_all_states() {
    let db = MockDatabase::new();
    let app_state = Arc::new(AppState::new_with_mock(db).await);

    let mut state_machine = app_state.watering_system.state_machine.write().await;

    // Start with a cycle
    state_machine.start_cycle(Cycle {
        id: 1,
        instructions: vec![(1, chrono::Duration::minutes(30))],
    });

    let mut wizard_mode = app_state.watering_system.wizard_mode.write().await;

    // Transition to Activating state
    state_machine.state = WateringState::Activating(1);

    // Test entering and leaving RainStart
    wizard_mode.handle_signal(EnvironmentalSignal::RainStart, &mut *state_machine);
    assert_eq!(state_machine.state, WateringState::Idle); // Paused due to rain
    assert!(wizard_mode.paused_state.is_some());

    wizard_mode.handle_signal(EnvironmentalSignal::RainStop, &mut *state_machine);
    assert!(wizard_mode.paused_state.is_none()); // Paused state cleared
    assert!(state_machine.cycle.is_some()); // Cycle restored
    assert_eq!(state_machine.state, WateringState::Activating(1)); // Resume properly

    // Test entering and leaving HighWind
    state_machine.state = WateringState::Watering(1); // Set state to watering
    wizard_mode.handle_signal(EnvironmentalSignal::HighWind, &mut *state_machine);
    assert_eq!(state_machine.state, WateringState::Idle); // Paused due to high wind
    assert!(wizard_mode.paused_state.is_some());

    wizard_mode.handle_signal(EnvironmentalSignal::LowWind, &mut *state_machine);
    assert!(wizard_mode.paused_state.is_none());
    assert!(state_machine.cycle.is_some());
    assert_eq!(state_machine.state, WateringState::Watering(1)); // Resume properly

    // Test transitions from idle and active states
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
    let db = MockDatabase::new();
    let app_state = Arc::new(AppState::new_with_mock(db).await);

    let mut state_machine = app_state.watering_system.state_machine.write().await;

    // Start a cycle and set state to Watering
    state_machine.start_cycle(Cycle {
        id: 1,
        instructions: vec![(1, chrono::Duration::minutes(30))],
    });
    state_machine.state = WateringState::Watering(1);

    let mut wizard_mode = app_state.watering_system.wizard_mode.write().await;

    // Test HighWind signal
    wizard_mode.handle_signal(EnvironmentalSignal::HighWind, &mut *state_machine);
    assert_eq!(state_machine.state, WateringState::Idle); // Irrigation paused
    assert!(wizard_mode.paused_state.is_some());

    // Test LowWind signal to resume
    wizard_mode.handle_signal(EnvironmentalSignal::LowWind, &mut *state_machine);
    assert!(state_machine.cycle.is_some()); // Cycle resumed
    assert_eq!(state_machine.state, WateringState::Watering(1)); // Resumes watering state
}

#[tokio::test]
async fn test_signal_handling_high_wind() {
    let db = MockDatabase::new();
    let app_state = Arc::new(AppState::new_with_mock(db).await);

    let mut state_machine = app_state.watering_system.state_machine.write().await;

    // Start a cycle and set state to Watering
    state_machine.start_cycle(Cycle {
        id: 1,
        instructions: vec![(1, chrono::Duration::minutes(30))],
    });
    state_machine.state = WateringState::Watering(1);

    let mut wizard_mode = app_state.watering_system.wizard_mode.write().await;

    // Test HighWind signal
    wizard_mode.handle_signal(EnvironmentalSignal::HighWind, &mut *state_machine);

    // Ensure the irrigation is paused
    assert_eq!(state_machine.state, WateringState::Idle); // Paused from watering state
    assert!(wizard_mode.paused_state.is_some());
    assert!(state_machine.cycle.is_none());
}
