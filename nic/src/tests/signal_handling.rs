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
